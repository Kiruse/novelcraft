use serde::{Deserialize, Serialize};
use tauri::AppHandle;
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_fs::FilePath;

#[derive(Debug, Serialize, Deserialize)]
pub struct ExportData {
  pub session: SessionData,
  pub pages: Vec<PageData>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SessionData {
  pub id: String,
  pub story_id: String,
  pub title: String,
  pub description: Option<String>,
  pub created_at: String,
  pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PageData {
  pub id: String,
  pub session_id: String,
  pub system: Option<String>,
  pub prompt: Option<String>,
  pub response: Option<String>,
  pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct FileFilter {
  pub name: String,
  pub extensions: Vec<String>,
}

fn file_path_to_string(fp: FilePath) -> String {
  match fp {
    FilePath::Path(p) => p.to_string_lossy().to_string(),
    FilePath::Url(u) => u.to_string(),
  }
}

#[tauri::command]
pub async fn export_session(
  _app: AppHandle,
  _session_id: String,
  file_path: String,
  data: ExportData,
) -> Result<(), String> {
  let content = serde_json::to_string_pretty(&data).map_err(|e| e.to_string())?;
  tokio::fs::write(&file_path, content)
    .await
    .map_err(|e| format!("Failed to write file: {}", e))?;
  Ok(())
}

#[tauri::command]
pub async fn import_session(file_path: String) -> Result<ExportData, String> {
  let content = tokio::fs::read_to_string(&file_path)
    .await
    .map_err(|e| format!("Failed to read file: {}", e))?;
  serde_json::from_str(&content).map_err(|e| format!("Failed to parse JSON: {}", e))
}

#[tauri::command]
pub async fn pick_file(
  app: AppHandle,
  filters: Option<Vec<FileFilter>>,
) -> Result<Option<String>, String> {
  let mut builder = app.dialog().file();

  if let Some(f) = filters {
    for filter in f {
      let exts: Vec<&str> = filter.extensions.iter().map(|s| s.as_str()).collect();
      builder = builder.add_filter(filter.name, &exts);
    }
  }

  let result = tokio::task::spawn_blocking(move || builder.blocking_pick_file())
    .await
    .map_err(|e| format!("Dialog error: {}", e))?;

  Ok(result.map(file_path_to_string))
}

#[tauri::command]
pub async fn pick_folder(app: AppHandle) -> Result<Option<String>, String> {
  let result = tokio::task::spawn_blocking(move || app.dialog().file().blocking_pick_folder())
    .await
    .map_err(|e| format!("Dialog error: {}", e))?;

  Ok(result.map(file_path_to_string))
}
