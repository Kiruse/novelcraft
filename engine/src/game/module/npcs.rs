use std::{collections::HashMap, sync::Arc};

use kiruklaw_agent_loop::{AgentToolset, toolset};
use log::warn;
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

  pub async fn context(&self, _state: GameState, f: &mut PromptFormatter) -> PromptifyResult {
    if self.npcs.is_empty() { return Ok(()) };

    f.writeline("# NPCs")?;
    for npc in self.npcs.values() {
      if npc.name.trim().is_empty() {
        warn!("NPC missing name");
        continue;
      }

      f.write("## ")?;
      f.writeline(&npc.name)?;

      if !npc.personality.trim().is_empty() {
        f.write("### Personality")?;
        f.writeline(&npc.personality)?;
      }

      // TODO: Physique & Apparel should be able to change during gameplay
      // And thus should be read from state rather than config
      if !npc.physique.trim().is_empty() {
        f.write("### Physique")?;
        f.writeline(&npc.physique)?;
      }

      if !npc.apparel.trim().is_empty() {
        f.write("### Apparel")?;
        f.writeline(&npc.apparel)?;
      }
    }

    Ok(())
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
