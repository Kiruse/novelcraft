use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameplayModule {
  pub name: String,
  pub summary: String,
  pub tools: Vec<GameplayModuleTool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameplayModuleTool {
  pub name: String,
  pub description: String,
  pub schema: String,
}
