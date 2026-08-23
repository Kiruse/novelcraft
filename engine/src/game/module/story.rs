use kiruklaw_agent_loop::{AgentToolset, toolset};
use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::game::module::{GameplayModuleCommons, GameStateView};
use crate::game::state::GameState;
use crate::markdown::{TodoList, TodoListDiff};
use crate::util::prompting::{PromptFormatter, PromptifyResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoryModule {}

impl StoryModule {
  pub const ID: &str = "story";

  fn id() -> String { Self::ID.to_string() }
}

impl GameplayModuleCommons for StoryModule {
  fn context(&self, state: &GameState, f: &mut PromptFormatter) -> PromptifyResult {
    let Some(module) = state.get(&Self::id()) else { return Ok(()) };
    let timeline = module.get::<String>(&Keypath::Timeline.to_keypath())
      .transpose()
      .unwrap_or_default();
    if let Some(timeline) = timeline {
      f.write("Timeline:")?;
      f.indented(|f| f.write(&timeline))?;
    }
    Ok(())
  }
}

#[toolset]
impl<'a> AgentToolset<GameStateView<'a>> for StoryModule {
  /// Manage the active timeline in the form of a TODO list. Checked-off items have been visited,
  /// unchecked items are outstanding. You may add new items (checked or unchecked), check off items,
  /// or remove unchecked items.
  /// @contents The new contents of the timeline.
  async fn manage_timeline(&self, ctx: &mut GameStateView<'a>, contents: String) -> Result<String, anyhow::Error> {
    let keypath = Keypath::Timeline.to_keypath();
    let existing = ctx.get::<String>(&Self::id(), &keypath)
      .transpose()?
      .unwrap_or_default();
    let existing = TodoList::parse(&existing)?;
    let new      = TodoList::parse(&contents)?;

    for item in &existing.diff(&new) {
      match item {
        TodoListDiff::Uncheck(item) if item.checked => {
          Err(AppError::illegal("cannot uncheck an already checked item"))?;
        }
        TodoListDiff::Remove(item) if item.checked => {
          Err(AppError::illegal("cannot remove an already checked item"))?;
        }
        _ => {}
      }
    }

    ctx.set(&keypath, &new.to_string())?;
    Ok("Timeline updated.".to_string())
  }
}

enum Keypath {
  Timeline,
}

impl Keypath {
  fn to_keypath(&self) -> Vec<String> {
    match self {
      Self::Timeline => vec!["timeline".to_string()],
    }
  }
}
