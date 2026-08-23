use kiruklaw_agent_loop::{AgentToolset, ConversationMessage, prompt, toolset};
use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::game::module::{GameStateView, GameplayModuleCommons, GameplayModuleInitContext};
use crate::util::discard_channel;
use crate::util::prompting::Promptify;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerModule {
  /// Player's starting apparel
  apparel: Option<String>,
}

impl PlayerModule {
  pub const ID: &str = "player";

  // fn id() -> String { Self::ID.to_string() }

  pub async fn init(&self, ctx: &mut GameplayModuleInitContext<'_>) -> Result<(), AppError> {
    if let Some(active_profile_id) = ctx.profiles.active_id.as_ref() {
      if let Some(profile) = ctx.profiles.profiles.iter().find(|profile| &profile.id == active_profile_id) {
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
        ctx.state.set(&Keypath::Apparel.to_keypath(), &content.trim())?;
      }
    }
    Ok(())
  }
}

impl GameplayModuleCommons for PlayerModule {}

#[toolset]
impl<'a> AgentToolset<GameStateView<'a>> for PlayerModule {}

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
