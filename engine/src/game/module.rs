use serde::{Deserialize, Serialize};
use specta::Type;

/// Gameplay modules are defined in LUA using mlua crate so they can be written & shared
/// w/o rebuilding the entire game engine.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct GameplayModule {
  pub name: String,
  pub summary: String,
  pub tools: Vec<GameplayModuleTool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct GameplayModuleTool {
  pub name: String,
  pub description: String,
  /// Serialized JSON schema
  // TODO: does this make sense?
  pub schema: String,
}
