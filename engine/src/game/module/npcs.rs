use std::{collections::HashMap, sync::Arc};

use kiruklaw_agent_loop::{AgentToolset, toolset};
use serde::{Deserialize, Serialize};

use crate::game::module::GameStateView;
use crate::game::state::GameState;
use crate::util::prompting::{PromptFormatter, PromptifyResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcsModule {
  pub npcs: Arc<HashMap<String, NpcDescriptor>>,
}

impl NpcsModule {
  pub const ID: &str = "npcs";

  // fn id() -> String { Self::ID.to_string() }

  pub async fn context(&self, state: GameState, f: &mut PromptFormatter) -> PromptifyResult {
    todo!()
  }
}

#[toolset]
impl AgentToolset<GameStateView> for NpcsModule {
  /// Update the named NPC's apparel.
  /// @name Name of the NPC whose apparel to update.
  /// @apparel Comma-separated list of apparel items.
  async fn update_apparel(
    &self,
    state: GameStateView,
    name: String,
    apparel: String,
  ) -> Result<String, anyhow::Error> {
    state.set(&Keypath::Apparel(name.clone()).to_keypath(), &apparel).await?;
    Ok(format!("Apparel of {name} successfully updated."))
  }
}

impl From<NpcsModule> for super::GameplayModule {
  fn from(value: NpcsModule) -> Self {
    Self::Npcs(value)
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcDescriptor {
  /// Full name of this character.
  pub name: String,
  /// Personality of this character, including favorites & despised.
  pub personality: String,
  /// Physique of this character, excluding apparel & equipment.
  pub physique: String,
  /// Apparel and/or equipment of this character, as comma-separated list preferably.
  pub apparel: String,
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
