use std::path::PathBuf;

use kiruklaw_agent_loop::ModelConfig;
use log::warn;
use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::paths;
use crate::util::{deserialize, ensure_dir, serialize};

/// Default host of llama.cpp
const DEFAULT_HOST: &str = "http://localhost:8888/v1";

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

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
#[serde(rename_all="snake_case")]
pub enum ModelUsage {
  Storyteller,
  Suggestions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Models {
  pub storyteller: ModelConfig,
  pub suggestions: ModelConfig,
}

impl Models {
  pub fn get_config(&self, usage: ModelUsage) -> &ModelConfig {
    match usage {
      ModelUsage::Storyteller => &self.storyteller,
      ModelUsage::Suggestions => &self.suggestions,
    }
  }

  pub fn all_configs(&self) -> Vec<&ModelConfig> {
    vec![&self.storyteller, &self.suggestions]
  }

  pub async fn load() -> Result<Models, AppError> {
    let path = Self::config_path()?;
    if path.exists() {
      crate::util::deserialize::<Models>(&path).await
        .or_else(|err| {
          warn!("Failed to deserialize models at {}: {} - initializing with defaults", path.display(), err);
          Ok(Models::default())
        })
    } else {
      Ok(Models::default())
    }
  }

  pub async fn save(&self) -> Result<(), AppError> {
    let path = Self::config_path()?;
    ensure_dir(&path).await?;
    crate::util::serialize(&path, self).await
  }

  fn config_path() -> Result<PathBuf, AppError> {
    paths::config_dir().map(|p| p.join("models.json"))
  }
}

impl Default for Models {
  fn default() -> Self {
    Self {
      storyteller: ModelConfig::OpenAi {
        base_url: DEFAULT_HOST.to_string(),
        model: "zai-org/glm-4.6v-flash".to_string(),
        api_key: "".to_string(),
      },
      suggestions: ModelConfig::OpenAi {
        base_url: DEFAULT_HOST.to_string(),
        model: "zai-org/glm-4.6v-flash".to_string(),
        api_key: "".to_string(),
      },
    }
  }
}
