use kiruklaw_agent_loop::{AgentToolset, ConversationMessage, prompt, toolset};
use serde::{Deserialize, Serialize};

use crate::error::EngineError;
use crate::game::module::{GameStateView, GameplayModuleInitContext};
use crate::game::state::{GameState, ModuleState};
use crate::util::discard_channel;
use crate::util::prompting::{PromptFormatter, Promptify, PromptifyResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerModule {
  /// Player's starting apparel
  apparel: Option<String>,
}

impl PlayerModule {
  pub const ID: &str = "player";

  fn id() -> String { Self::ID.to_string() }

  pub async fn init(&self, ctx: &mut GameplayModuleInitContext<'_>) -> Result<(), EngineError> {
    if let Some(profile) = ctx.profile {
      let mut fields = profile.fields.clone();
      fields.insert("Name".to_string(), profile.name.clone());
      let (msg, _) = prompt(
        &ctx.model_cfg,
        &[],
        vec![
          ConversationMessage::system("Following is the user's RPG character profile. Please extract the player character's apparel as a short, comma-separated list.".to_string()).into(),
          ConversationMessage::user(fields.to_prompt()?).into(),
        ],
        discard_channel(),
      ).await?;
      let ConversationMessage::Assistant { content, .. } = msg else { unreachable!() };
      ctx.state.set(&Keypath::Apparel.to_keypath(), &content.trim()).await?;
    }
    Ok(())
  }

  pub async fn context(&self, state: GameState, f: &mut PromptFormatter) -> PromptifyResult {
    if let Some(profile) = &state.profile {
      let module = state.get(&Self::id()).expect("Missing player module state");
      f.writeline("# Player Character Profile")?;

      f.write("Name: ")?;
      f.writeline(&profile.name)?;

      for (k, v) in &profile.fields {
        match k.trim().to_lowercase().as_str() {
          "apparel" => {
            f.write("Apparel: ")?;
            let apparel = Self::get_opt_field(module, Keypath::Apparel)
              .await
              .unwrap_or("None".to_string());
            f.writeline(&apparel)?;
          }
          _ => {
            f.write(k)?;
            f.write(": ")?;
            f.writeline(v)?;
          }
        }
      }
    }
    Ok(())
  }

  #[inline(always)]
  async fn get_opt_field(state: &ModuleState, keypath: Keypath) -> Option<String> {
    state.get(&keypath.to_keypath()).await.transpose().unwrap_or_default()
  }
}

impl From<PlayerModule> for super::GameplayModule {
  fn from(value: PlayerModule) -> Self {
    Self::Player(value)
  }
}

#[toolset]
impl AgentToolset<GameStateView> for PlayerModule {}

#[derive(Debug, Clone)]
enum Keypath {
  Apparel,
}

impl Keypath {
  fn to_keypath(&self) -> Vec<String> {
    match self {
      Self::Apparel => vec!["apparel".to_string()],
    }
  }
}
