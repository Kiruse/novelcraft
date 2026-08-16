use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::commands::paths;
use crate::util::{deserialize, serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NovelCraftConfig {
  pub max_agent_steps: u8,
}

impl NovelCraftConfig {
  pub async fn load() -> Result<NovelCraftConfig, AppError> {
    deserialize(&Self::default_path()?).await
  }

  pub async fn save(&self) -> Result<(), AppError> {
    serialize(&Self::default_path()?, self).await
  }

  fn default_path() -> Result<PathBuf, AppError> {
    Ok(paths::config_dir()?.join("config.json"))
  }
}

impl Default for NovelCraftConfig {
  fn default() -> Self {
    Self {
      max_agent_steps: 10,
    }
  }
}
