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

impl Default for NovelCraftConfig {
  fn default() -> Self {
    Self {
      max_agent_steps: 10,
    }
  }
}

fn config_path(app: &AppHandle) -> Result<PathBuf, AppError> {
  Ok(app.path().app_config_dir()?.join("config.json"))
}

pub async fn load_config(app: &AppHandle) -> Result<NovelCraftConfig, AppError> {
  deserialize(&config_path(app)?).await
}

pub async fn save_config(app: &AppHandle, config: &NovelCraftConfig) -> Result<(), AppError> {
  serialize(&config_path(app)?, config).await
}
