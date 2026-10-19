use std::path::PathBuf;

use exhaustive_map::Finite;
use kiruklaw_agent_loop::AgentLoop;
pub use kiruklaw_agent_loop::ModelConfig;
use serde::{Deserialize, Serialize};

use crate::error::EngineError;
use crate::game::profile::ProfileV1;
use crate::game::state::GameStateView;
use crate::paths;
use crate::util::{deserialize, deserialize_sync, serialize};

/// Default host of llama.cpp
pub const DEFAULT_HOST: &str = "https://openrouter.ai/api/v1";
pub const DEFAULT_MODEL: &str = "google/gemma-4-26b-a4b-it:free";

pub const DEFAULT_SYSTEM_PROMPT: &str = r#"
You are a Dungeon Master in a novel, modernized text adventure game session.
Your objective is to guide the player through the story, managing your memory
of events, updating the state of the player & NPCs, planning & tracking the
timeline, and assuming the roles of NPCs in dialogs and interactions.

Maintain your long-term memory with the memory tools: note down important
events of the current chapter as concise bullet points as they happen, and
whenever the story reaches a significant shift in plot, location or cast,
close the current chapter under a fitting title and begin a new one.

NEVER speak for the player.
NEVER perform actions for the player.
ALWAYS keep your responses short.
ALWAYS use tool calls to interact with the game engine's deterministic state and
to retrieve necessary knowledge.
"#;

/// Default number of turns between memory compressions.
pub const DEFAULT_SUMMARY_THRESHOLD: usize = 50;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NovelCraftConfig {
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
  /// User profiles.
  pub profiles: Vec<ProfileV1>,
  /// Active user profile ID.
  pub active_profile: Option<String>,
}

impl NovelCraftConfig {
  fn default_summary_threshold() -> usize {
    DEFAULT_SUMMARY_THRESHOLD
  }
  pub async fn load() -> Result<NovelCraftConfig, EngineError> {
    deserialize(&Self::default_path()?).await
  }

  pub fn load_sync() -> Result<NovelCraftConfig, EngineError> {
    deserialize_sync(&Self::default_path()?)
  }

  pub async fn save(&self) -> Result<(), EngineError> {
    serialize(&Self::default_path()?, self).await
  }

  fn default_path() -> Result<PathBuf, EngineError> {
    Ok(paths::config_dir()?.join("config.json"))
  }

  /// Update the given `agent_loop` with values from this configuration.
  pub(crate) fn contribute(&self, agent_loop: Option<&mut AgentLoop<GameStateView>>) {
    let Some(agent_loop) = agent_loop else { return };
    agent_loop.max_steps = self.max_agent_steps;
    agent_loop.persona = Some(self.system_prompt.clone());
    agent_loop.model = self.models[ModelPurpose::DungeonMaster].clone();
  }
}

impl Default for NovelCraftConfig {
  fn default() -> Self {
    Self {
      max_agent_steps: 10,
      system_prompt: DEFAULT_SYSTEM_PROMPT.trim().to_string(),
      models: Models::default(),
      summary_threshold: Self::default_summary_threshold(),
      profiles: vec![],
      active_profile: None,
    }
  }
}

#[derive(Debug, Copy, Clone, Finite, Serialize, Deserialize)]
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
      Self::Suggestions   => "Suggestions",
      Self::Summarizer    => "Summarizer",
    }
  }
}

pub type Models = exhaustive_map::ExhaustiveMap<ModelPurpose, ModelConfig>;
