use std::path::PathBuf;

use kiruklaw_agent_loop::ModelConfig;
use log::warn;
use serde::{Deserialize, Serialize};

use crate::error::EngineError;
use crate::game::profile::ProfileV1;
use crate::paths;
use crate::util::{deserialize, ensure_dir, serialize};

/// Default host of llama.cpp
const DEFAULT_HOST: &str = "http://localhost:8888/v1";

const DEFAULT_SYSTEM_PROMPT: &str = r#"
You are a Dungeon Master in a novel, modernized text adventure game session.
Your objective is to guide the player through the story, managing your memory
of events, updating the state of the player & NPCs, planning & tracking the
timeline, and assuming the roles of NPCs in dialogs and interactions.

NEVER speak for the player.
NEVER perform actions for the player.
ALWAYS keep your responses short.
ALWAYS use tool calls to interact with the game engine's deterministic state and
to retrieve necessary knowledge.
"#;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NovelCraftConfig {
  /// Max agent loop steps. Defaults to 10.
  pub max_agent_steps: u8,
  /// Persona/system prompt of the main dungeon master agent.
  pub system_prompt: String,
  /// Configured models by usage.
  pub models: Models,
  /// User profiles.
  pub profiles: Vec<ProfileV1>,
  /// Active user profile ID.
  pub active_profile: Option<String>,
}

impl NovelCraftConfig {
  pub async fn load() -> Result<NovelCraftConfig, EngineError> {
    deserialize(&Self::default_path()?).await
  }

  pub async fn save(&self) -> Result<(), EngineError> {
    serialize(&Self::default_path()?, self).await
  }

  fn default_path() -> Result<PathBuf, EngineError> {
    Ok(paths::config_dir()?.join("config.json"))
  }
}

impl Default for NovelCraftConfig {
  fn default() -> Self {
    Self {
      max_agent_steps: 10,
      system_prompt: DEFAULT_SYSTEM_PROMPT.trim().to_string(),
      models: Models::default(),
      profiles: vec![],
      active_profile: None,
    }
  }
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
#[serde(rename_all="snake_case")]
pub enum ModelUsage {
  DungeonMaster,
  Suggestions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Models {
  pub dungeon_master: ModelConfig,
  pub suggestions: ModelConfig,
}

impl Models {
  pub fn get_config(&self, usage: ModelUsage) -> &ModelConfig {
    match usage {
      ModelUsage::DungeonMaster => &self.dungeon_master,
      ModelUsage::Suggestions => &self.suggestions,
    }
  }

  pub fn all_configs(&self) -> Vec<&ModelConfig> {
    vec![&self.dungeon_master, &self.suggestions]
  }

  pub async fn load() -> Result<Models, EngineError> {
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

  pub async fn save(&self) -> Result<(), EngineError> {
    let path = Self::config_path()?;
    ensure_dir(&path).await?;
    crate::util::serialize(&path, self).await
  }

  fn config_path() -> Result<PathBuf, EngineError> {
    paths::config_dir().map(|p| p.join("models.json"))
  }
}

impl Default for Models {
  fn default() -> Self {
    Self {
      dungeon_master: ModelConfig::OpenAi {
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
