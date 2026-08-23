use kiruklaw_agent_loop::{AgentToolset, ModelConfig};
use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::game::profile::Profiles;
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
  fn as_toolset(&self) -> &dyn AgentToolset<GameStateView<'_>> {
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

  pub async fn init(&self, ctx: &mut GameplayModuleInitContext<'_>) -> Result<(), AppError> {
    match self {
      Self::Player(module) => module.init(ctx).await,
      _ => Ok(()),
    }
  }

  pub async fn handle<'a>(&'a self, ctx: &'a mut GameStateView<'a>, tool_name: &str, args: String) -> String {
    self.as_toolset().handle(ctx, tool_name, args).await
  }

  /// Retrieve agentic context of this module as key => value pairs.
  pub fn context(&self, state: &GameState, f: &mut PromptFormatter) -> PromptifyResult {
    match self {
      _ => Ok(())
    }
  }
}

pub trait GameplayModuleCommons {
  /// Retrieve agentic context of this module as key => value pairs.
  #[allow(unused)]
  fn context(&self, state: &GameState, f: &mut PromptFormatter) -> PromptifyResult { Ok(()) }
}

#[derive(Debug)]
pub struct GameplayModuleInitContext<'a> {
  pub state: &'a mut GameStateView<'a>,
  pub model_cfg: &'a ModelConfig,
  pub profiles: &'a Profiles,
}
