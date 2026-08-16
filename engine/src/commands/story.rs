use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::commands::paths as cmd_paths;
use crate::error::AppError;
use crate::util;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoryEntry {
  pub version: u32,
  pub id: String,
  pub title: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub description: Option<String>,
  pub config: serde_json::Value,
  pub created_at: String,
  pub updated_at: String,
}

fn stories_dir() -> Result<PathBuf, AppError> {
  cmd_paths::stories_dir()
}

pub async fn story_get(id: String) -> Result<Option<StoryEntry>, AppError> {
  let dir = stories_dir()?;
  let path = dir.join(format!("{}.json", id));
  if path.exists() {
    Ok(Some(util::deserialize(&path).await?))
  } else {
    Ok(None)
  }
}

pub async fn story_save(story: StoryEntry) -> Result<(), AppError> {
  let dir = stories_dir()?;
  util::ensure_dir(&dir).await?;
  let path = dir.join(format!("{}.json", story.id));
  util::serialize(&path, &story).await
}
