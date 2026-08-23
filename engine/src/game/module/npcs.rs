use kiruklaw_agent_loop::{AgentToolset, toolset};
use serde::{Deserialize, Serialize};

use crate::game::{module::{GameStateView, GameplayModuleCommons}, state::GameState};
use crate::util::prompting::{PromptFormatter, PromptifyResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcsModule {
  // todo
}

impl NpcsModule {
  pub const ID: &str = "npcs";

  // fn id() -> String { Self::ID.to_string() }
}

impl GameplayModuleCommons for NpcsModule {
  fn context(&self, state: &GameState, f: &mut PromptFormatter) -> PromptifyResult {
    todo!()
  }
}

#[toolset]
impl<'a> AgentToolset<GameStateView<'a>> for NpcsModule {
  /// Update the named NPC's apparel.
  /// @name Name of the NPC whose apparel to update.
  /// @apparel Comma-separated list of apparel items.
  async fn update_apparel(
    &self,
    state: &mut GameStateView<'a>,
    name: String,
    apparel: String,
  ) -> Result<String, anyhow::Error> {
    state.set(&Keypath::Apparel(name.clone()).to_keypath(), &apparel)?;
    Ok(format!("Apparel of {name} successfully updated."))
  }
}

#[derive(Debug, Clone)]
enum Keypath {
  Apparel(String),
}

impl Keypath {
  fn to_keypath(self) -> Vec<String> {
    match self {
      Self::Apparel(name) => vec![name, "apparel".to_string()],
    }
  }
}
