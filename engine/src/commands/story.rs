use serde::{Deserialize, Serialize};
use specta::Type;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

use crate::error::AppError;
use crate::util;

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct StoryEntry {
  pub version: u32,
  pub id: String,
  pub title: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub description: Option<String>,
  #[specta(type = specta_typescript::Any)]
  pub config: serde_json::Value,
  pub created_at: String,
  pub updated_at: String,
}

fn stories_dir(app: &AppHandle) -> Result<PathBuf, AppError> {
  Ok(
    app
      .path()
      .app_data_dir()
      .map_err(|e| AppError::internal(e.to_string()))?
      .join("stories"),
  )
}

#[tauri::command]
#[specta::specta]
pub async fn story_get(app: AppHandle, id: String) -> Result<Option<StoryEntry>, AppError> {
  let dir = stories_dir(&app)?;
  let path = dir.join(format!("{}.json", id));
  if path.exists() {
    Ok(Some(util::deserialize(&path).await?))
  } else {
    Ok(None)
  }
}

#[tauri::command]
#[specta::specta]
pub async fn story_save(app: AppHandle, story: StoryEntry) -> Result<(), AppError> {
  let dir = stories_dir(&app)?;
  util::ensure_dir(&dir).await?;
  let path = dir.join(format!("{}.json", story.id));
  util::serialize(&path, &story).await
}
