use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{AppHandle, Manager};

use crate::error::AppError;
use crate::util::{deserialize, serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct NovelCraftConfig {
  pub max_agent_steps: u8,
}

impl NovelCraftConfig {
  pub async fn load(app: &AppHandle) -> Result<NovelCraftConfig, AppError> {
    deserialize(&Self::default_path(app)?).await
  }

  pub async fn save(&self, app: &AppHandle) -> Result<(), AppError> {
    serialize(&Self::default_path(app)?, self).await
  }

  fn default_path(app: &AppHandle) -> Result<PathBuf, AppError> {
    Ok(app.path().app_config_dir()?.join("config.json"))
  }
}

impl Default for NovelCraftConfig {
  fn default() -> Self {
    Self {
      max_agent_steps: 10,
    }
  }
}
