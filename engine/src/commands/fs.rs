use std::path::PathBuf;
use tauri::{AppHandle, Manager};

use crate::error::AppError;
use crate::util::canonical_path;

#[tauri::command]
#[specta::specta]
pub async fn datapath(app: AppHandle, path: String) -> Result<String, AppError> {
  let basepath = app
    .path()
    .app_data_dir()
    .map_err(|e| AppError::internal(e.to_string()))?;
  let basepath = canonical_path(&basepath)?;

  let path = basepath.join(PathBuf::from(path));
  if !path.starts_with(basepath) {
    return Err(AppError::Path("Path out of bounds".to_string()));
  }

  Ok(path.to_string_lossy().to_string())
}
