use kiruklaw_agent_loop::{AgentToolset, ModelConfig, Toolset};
use serde::{Deserialize, Serialize};

use crate::error::EngineError;
use crate::game::profile::ProfileV1;
use crate::game::state::{GameState, GameStateView};
use crate::util::prompting::{PromptFormatter, PromptifyResult};

pub mod story;
pub mod npcs;
pub mod player;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GameplayModule {
  /// Story module provides authored events & trajectory planning
  Story(story::StoryModule),

  /// NPCs module provides persistency for NPCs
  Npcs(npcs::NpcsModule),

  /// Player module manages player-relevant state. Initial state
  /// is populated from profiles + story module.
  Player(player::PlayerModule),
}

impl GameplayModule {
  pub fn as_toolset(&self) -> &dyn AgentToolset<GameStateView> {
    match self {
      Self::Story(module)  => module,
      Self::Npcs(module)   => module,
      Self::Player(module) => module,
    }
  }

  pub fn id(&self) -> &'static str {
    match self {
      Self::Story(_)  => story::StoryModule::ID,
      Self::Npcs(_)   => npcs::NpcsModule::ID,
      Self::Player(_) => player::PlayerModule::ID,
    }
  }

  pub async fn init(&self, ctx: &mut GameplayModuleInitContext<'_>) -> Result<(), EngineError> {
    match self {
      Self::Player(module) => module.init(ctx).await,
      _ => Ok(()),
    }
  }

  pub async fn handle<'a>(&'a self, ctx: GameStateView, tool_name: &str, args: String) -> String {
    self.as_toolset().handle(ctx, tool_name, args).await
  }

  /// Retrieve agentic context of this module as key => value pairs.
  pub async fn context(&self, state: GameState, f: &mut PromptFormatter) -> PromptifyResult {
    match self {
      Self::Story(module)  => module.context(state, f).await,
      Self::Npcs(module)   => module.context(state, f).await,
      Self::Player(module) => module.context(state, f).await,
    }
  }

  pub(crate) fn toolset<'a>(self) -> Toolset<GameStateView> {
    Toolset::Immutable(match self {
      Self::Npcs(module)   => Box::new(module),
      Self::Player(module) => Box::new(module),
      Self::Story(module)  => Box::new(module),
    })
  }
}

#[derive(Debug)]
pub struct GameplayModuleInitContext<'a> {
  pub state: GameStateView,
  pub model_cfg: &'a ModelConfig,
  pub profile: Option<&'a ProfileV1>,
}
