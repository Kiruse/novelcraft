use serde::{Deserialize, Serialize};
use specta::Type;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

use crate::error::AppError;
use crate::util;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoreEntry {
  pub version: u32,
  pub id: String,
  pub story_id: String,
  pub title: String,
  pub content: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub tags: Option<Vec<String>>,
  pub created_at: String,
  pub updated_at: String,
}

#[derive(Debug, Serialize, Type)]
pub struct LoreQueryResult {
  pub id: String,
  pub title: String,
  pub content: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub tags: Option<Vec<String>>,
}

fn lore_dir(app: &AppHandle) -> Result<PathBuf, AppError> {
  Ok(
    app
      .path()
      .app_data_dir()
      .map_err(|e| AppError::internal(e.to_string()))?
      .join("lore"),
  )
}

#[tauri::command]
#[specta::specta]
pub async fn lore_query(
  app: AppHandle,
  story_id: String,
  query: String,
) -> Result<Vec<LoreQueryResult>, AppError> {
  let dir = lore_dir(&app)?;
  if !dir.exists() {
    return Ok(Vec::new());
  }

  let mut entries = tokio::fs::read_dir(&dir).await?;
  let mut results: Vec<(String, LoreEntry)> = Vec::new();
  let pattern = query.to_lowercase();

  while let Some(entry) = entries.next_entry().await? {
    let path = entry.path();
    if path.extension().is_some_and(|e| e == "json") {
      match util::deserialize::<LoreEntry>(&path).await {
        Ok(lore) if lore.story_id == story_id => {
          let title_match = lore.title.to_lowercase().contains(&pattern);
          let content_match = lore.content.to_lowercase().contains(&pattern);
          if title_match || content_match {
            results.push((lore.updated_at.clone(), lore));
          }
        }
        _ => continue,
      }
    }
  }

  results.sort_by(|a, b| b.0.cmp(&a.0));
  results.truncate(10);

  Ok(
    results
      .into_iter()
      .map(|(_, lore)| LoreQueryResult {
        id: lore.id,
        title: lore.title,
        content: lore.content,
        tags: lore.tags,
      })
      .collect(),
  )
}
