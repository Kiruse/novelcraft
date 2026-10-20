use kiruklaw_agent_loop::{AgentToolset, ConversationMessage, prompt, toolset};
use serde::{Deserialize, Serialize};

use crate::error::EngineError;
use crate::game::module::{GameStateView, GameplayModuleInitContext};
use crate::game::state::{GameState, ModuleState};
use crate::util::discard_channel;
use crate::util::prompting::{PromptFormatter, Promptify, PromptifyResult};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PlayerModule {
  /// Player's starting apparel
  apparel: Option<String>,
}

impl PlayerModule {
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
        None,
        discard_channel(),
      ).await?;
      let ConversationMessage::Assistant { content, .. } = msg else { unreachable!() };
      ctx.state.set(&Keypath::Apparel.to_keypath(), &content.trim()).await?;
    }
    Ok(())
  }

  /// Render the agent-facing context: only the derived apparel this module
  /// manages at runtime. The player profile itself is rendered by the
  /// engine-level system message — never by a module.
  pub async fn context(&self, state: GameState, f: &mut PromptFormatter) -> PromptifyResult {
    let module = state.get(&Self::id()).expect("Missing player module state");
    let apparel = Self::get_opt_field(module, Keypath::Apparel)
      .await
      .unwrap_or_default();
    if !apparel.trim().is_empty() {
      f.writeline("## Player Character")?;
      f.write("Apparel: ")?;
      f.writeline(&apparel)?;
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

#[toolset(strip_suffix = "module")]
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

#[cfg(test)]
mod tests {
  use super::*;
  use crate::game::state::GameState;

  fn state() -> GameState {
    GameState::new(Default::default(), vec![PlayerModule::ID.to_string()], None).unwrap()
  }

  #[tokio::test]
  async fn context_renders_derived_apparel_only() {
    let state = state();
    state
      .view(PlayerModule::ID.to_string())
      .set(&Keypath::Apparel.to_keypath(), &"iron helmet, wool cloak")
      .await
      .unwrap();

    let mut f = PromptFormatter::new();
    PlayerModule::default().context(state, &mut f).await.unwrap();
    let text = f.finish();

    assert!(text.contains("# Player Character"));
    assert!(text.contains("Apparel: iron helmet, wool cloak"));
    // The raw profile never appears here — the engine renders it.
    assert!(!text.contains("Name:"));
  }

  #[tokio::test]
  async fn context_is_empty_without_apparel() {
    let mut f = PromptFormatter::new();
    PlayerModule::default().context(state(), &mut f).await.unwrap();
    assert!(f.finish().trim().is_empty());
  }
}
