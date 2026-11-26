use std::path::PathBuf;

use exhaustive_map::Finite;
use kiruklaw_agent_loop::AgentLoop;
use serde::{Deserialize, Serialize};

use crate::error::EngineError;
use crate::game::profile::ProfileV1;
use crate::game::state::GameStateView;
use crate::paths;
use crate::util::fs::{deserialize, serialize};
use crate::util::json::{json, JsonValue, SerdeJsonExt};

/// Default host of the NovelCraft hosted-inference API
pub const DEFAULT_HOST: &str = "https://api.novelcraft.quest";
pub const DEFAULT_MODEL: &str = "google/gemma-4-26b-a4b-it";

pub const DEFAULT_SYSTEM_PROMPT: &str = r#"
You are a Dungeon Master in a novel, modernized text adventure game session. \
Your objective is to guide the player through the story, managing your memory \
of events, updating the state of the player & NPCs, planning & tracking the \
timeline, and assuming the roles of NPCs in dialogs and interactions.

Maintain your long-term memory with the memory tools: note down important \
events of the current chapter as concise bullet points as they happen, and \
whenever the story reaches a significant shift in plot, location or cast, \
close the current chapter under a fitting title and begin a new one.

NEVER speak for the player.
NEVER perform actions for the player.
ALWAYS keep your responses short.
ALWAYS use tool calls to interact with the game engine's deterministic state and \
to retrieve necessary knowledge.
"#;

/// Default number of turns between memory compressions.
pub const DEFAULT_SUMMARY_THRESHOLD: usize = 50;

/// Default assumed context window (in tokens) of the dungeon master model.
pub const DEFAULT_CONTEXT_LENGTH: u64 = 8192;

pub trait ConfigProvider {
  fn get_config(&self) -> &NovelCraftConfig;
  fn get_config_mut(&mut self) -> &mut NovelCraftConfig;

  /// Update the given `agent_loop` with values from this configuration.
  /// Model resolution happens per call via [`crate::game::engine::ModelProvider`]
  /// — the credentials of the [`Login`](ModelConfig::Login) variant are
  /// the provider's concern, not the loop's.
  fn contribute(&self, agent_loop: Option<&mut AgentLoop<GameStateView>>) {
    let Some(agent_loop) = agent_loop else { return };
    agent_loop.max_steps = self.get_config().max_agent_steps;
    agent_loop.persona = Some(self.get_config().system_prompt.clone());
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NovelCraftConfig {
  /// Version of this configuration. Defaults to [`NovelCraftConfig::VERSION`]
  /// for config files predating the field.
  #[serde(default = "NovelCraftConfig::default_version")]
  pub version: u8,
  /// Max agent loop steps. Defaults to 10.
  pub max_agent_steps: u8,
  /// Persona/system prompt of the main dungeon master agent.
  pub system_prompt: String,
  /// Configured models by usage.
  pub models: Models,
  /// Rolling-summary compression threshold in turns. Once the retained
  /// short-term history exceeds twice this value, the oldest
  /// `summary_threshold` turns are compressed into the rolling summary.
  #[serde(default = "NovelCraftConfig::default_summary_threshold")]
  pub summary_threshold: usize,
  /// Assumed context window (in tokens) of the dungeon master model, used
  /// to render how much of the window the conversation is filling. The
  /// server does not report this portably, so it is set by hand.
  #[serde(default = "NovelCraftConfig::default_context_length")]
  pub context_length: u64,
  /// User profiles. Defaults to an empty list for config files predating
  /// profiles.
  #[serde(default)]
  pub profiles: Vec<ProfileV1>,
  /// Active user profile ID.
  #[serde(default)]
  pub active_profile: Option<String>,
}

impl NovelCraftConfig {
  const VERSION: u8 = 1;

  fn default_summary_threshold() -> usize {
    DEFAULT_SUMMARY_THRESHOLD
  }
  fn default_context_length() -> u64 {
    DEFAULT_CONTEXT_LENGTH
  }
  fn default_version() -> u8 {
    Self::VERSION
  }

  pub async fn load_json() -> Result<JsonValue, EngineError> {
    Self::migrate(deserialize(&Self::default_path()?).await?)
  }

  /// Fold legacy config layouts into the current schema. Configs already
  /// carrying the current field shapes are left untouched.
  fn migrate(mut original: JsonValue) -> Result<JsonValue, EngineError> {
    let get_version = |json: &JsonValue| json
      .get("version")
      .and_then(|v| v.as_u64())
      // Legacy configs didn't feature any version field so we consider them v0
      .unwrap_or(0) as u8;
    let mut v = get_version(&original);
    while v != Self::VERSION {
      Self::migrate_step(&mut original, v)?;
      v = get_version(&original);
    }
    Ok(original)
  }

  fn migrate_step(original: &mut JsonValue, v: u8) -> Result<(), EngineError> {
    match v {
      0 => Self::migrate_v0(original),
      x if x > Self::VERSION =>
        Err(EngineError::Validation(format!("Unknown future version {v}, current is {}", Self::VERSION))),
      _ => Err(EngineError::Validation(format!("Invalid config version {v}"))),
    }
  }

  fn migrate_v0(original: &mut JsonValue) -> Result<(), EngineError> {
    // from v0 (legacy, no version in obj) to v1, the only change are models

    let obj = original.as_object_mut()
      .ok_or_else(|| EngineError::illegal("config must be an object"))?;

    match obj.get_mut("models") {
      Some(models) => {
        let models = models.as_object_mut()
          .ok_or(EngineError::illegal("invalid models config"))?;
        for model in models.values_mut() {
          model.get_str("type")
            .filter(|v| *v == "open_ai")
            .ok_or(EngineError::illegal("invalid ModelConfig type"))?;
          let new_conf = ModelConfig::OpenAiCompatible {
            base_url: model.get_str("base_url").unwrap_or(DEFAULT_HOST).to_string(),
            api_key: model.get_str("api_key").unwrap_or_default().to_string(),
            model: model.get_str("model").unwrap_or(DEFAULT_MODEL).to_string(),
          };
          *model = serde_json::json!(new_conf);
        }
      }
      None => {
        obj.insert("models".to_string(), json!(Models::default()));
      }
    }
    obj.insert("version".to_string(), json!(1u8));
    Ok(())
  }

  pub async fn save(&self) -> Result<(), EngineError> {
    serialize(&Self::default_path()?, self).await
  }

  pub fn default_path() -> Result<PathBuf, EngineError> {
    Ok(paths::config_dir()?.join("config.json"))
  }
}

impl Default for NovelCraftConfig {
  fn default() -> Self {
    Self {
      version: Self::VERSION,
      max_agent_steps: 10,
      system_prompt: DEFAULT_SYSTEM_PROMPT.trim().to_string(),
      models: Models::default(),
      summary_threshold: Self::default_summary_threshold(),
      context_length: Self::default_context_length(),
      profiles: vec![],
      active_profile: None,
    }
  }
}

impl ConfigProvider for NovelCraftConfig {
  fn get_config(&self) -> &NovelCraftConfig {
    self
  }
  fn get_config_mut(&mut self) -> &mut NovelCraftConfig {
    self
  }
}

#[derive(Debug, Copy, Clone, PartialEq, Finite, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelPurpose {
  DungeonMaster,
  Suggestions,
  Summarizer,
}

impl ModelPurpose {
  pub fn as_str(&self) -> &'static str {
    match self {
      Self::DungeonMaster => "Dungeon Master",
      Self::Suggestions => "Suggestions",
      Self::Summarizer => "Summarizer",
    }
  }
}

pub type Models = exhaustive_map::ExhaustiveMap<ModelPurpose, ModelConfig>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ModelConfig {
  Login {
    model: String,
  },
  OpenAiCompatible {
    base_url: String,
    api_key: String,
    model: String,
  },
}

impl Default for ModelConfig {
  fn default() -> Self {
    Self::Login {
      model: DEFAULT_MODEL.to_string(),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  /// A v0 (legacy) config: kiruklaw's internally-tagged model entries
  /// (`{"type": "open_ai", ...}`), no `version` field, plus a GUI-era
  /// unknown field — all rewritten into the current schema.
  #[test]
  fn migrate_v0_rewrites_legacy_models() {
    let legacy = serde_json::json!({
      "max_agent_steps": 10,
      "system_prompt": "persona",
      "models": {
        "dungeon_master": {
          "type": "open_ai",
          "base_url": "http://localhost:8888/v1",
          "api_key": "sk-test",
          "model": "test-model"
        },
        "suggestions": { "type": "open_ai", "base_url": "x", "api_key": "", "model": "m" },
        "summarizer": { "type": "open_ai", "base_url": "x", "api_key": "", "model": "m" }
      },
      "onboarded": true
    });

    let migrated = NovelCraftConfig::migrate(legacy).unwrap();
    let config: NovelCraftConfig = serde_json::from_value(migrated).unwrap();

    assert_eq!(config.version, NovelCraftConfig::VERSION);
    assert_eq!(
      config.models[ModelPurpose::DungeonMaster],
      ModelConfig::OpenAiCompatible {
        base_url: "http://localhost:8888/v1".to_string(),
        api_key: "sk-test".to_string(),
        model: "test-model".to_string(),
      }
    );
    assert_eq!(
      config.models[ModelPurpose::Summarizer],
      ModelConfig::OpenAiCompatible {
        base_url: "x".to_string(),
        api_key: String::new(),
        model: "m".to_string(),
      }
    );
  }

  /// A legacy config without a `models` map is seeded with the module
  /// defaults (hosted inference on the default model) and bumped to the
  /// current version.
  #[test]
  fn migrate_v0_seeds_missing_models() {
    let legacy = serde_json::json!({
      "max_agent_steps": 10,
      "system_prompt": "persona"
    });

    let migrated = NovelCraftConfig::migrate(legacy).unwrap();
    let config: NovelCraftConfig = serde_json::from_value(migrated).unwrap();

    assert_eq!(config.version, NovelCraftConfig::VERSION);
    assert_eq!(
      config.models[ModelPurpose::DungeonMaster],
      ModelConfig::default()
    );
    assert_eq!(
      config.models[ModelPurpose::Summarizer],
      ModelConfig::default()
    );
  }

  /// A current-version config passes through the migration untouched —
  /// its internally-tagged model entries deserialize as-is.
  #[test]
  fn migrate_leaves_current_configs_untouched() {
    let raw = serde_json::json!({
      "version": 1,
      "max_agent_steps": 10,
      "system_prompt": "persona",
      "models": {
        "dungeon_master": { "type": "Login", "model": "hosted-model" },
        "suggestions": { "type": "Login", "model": "hosted-model" },
        "summarizer": {
          "type": "OpenAiCompatible", "base_url": "x", "api_key": "", "model": "m"
        }
      },
      "summary_threshold": 50,
      "context_length": 8192,
      "profiles": [],
      "active_profile": null
    });

    let migrated = NovelCraftConfig::migrate(raw).unwrap();
    let config: NovelCraftConfig = serde_json::from_value(migrated).unwrap();

    assert_eq!(
      config.models[ModelPurpose::DungeonMaster],
      ModelConfig::Login { model: "hosted-model".to_string() }
    );
    assert_eq!(
      config.models[ModelPurpose::Summarizer],
      ModelConfig::OpenAiCompatible {
        base_url: "x".to_string(),
        api_key: String::new(),
        model: "m".to_string(),
      }
    );
  }

  /// Config files from a future version are rejected outright.
  #[test]
  fn migrate_rejects_future_versions() {
    let future = serde_json::json!({
      "version": NovelCraftConfig::VERSION + 1,
      "max_agent_steps": 10,
      "system_prompt": "persona",
      "models": {}
    });

    assert!(NovelCraftConfig::migrate(future).is_err());
  }

  /// A legacy model entry with an unexpected tag aborts the migration —
  /// the config is considered manually corrupted.
  #[test]
  fn migrate_v0_rejects_unknown_model_types() {
    let legacy = serde_json::json!({
      "max_agent_steps": 10,
      "system_prompt": "persona",
      "models": {
        "dungeon_master": { "type": "garbage", "model": "m" },
        "suggestions": { "type": "open_ai", "base_url": "x", "api_key": "", "model": "m" },
        "summarizer": { "type": "open_ai", "base_url": "x", "api_key": "", "model": "m" }
      }
    });

    assert!(NovelCraftConfig::migrate(legacy).is_err());
  }
}
