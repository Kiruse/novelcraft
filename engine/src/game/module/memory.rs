use kiruklaw_agent_loop::{AgentToolset, toolset};
use serde::{Deserialize, Serialize};

use crate::error::EngineError;
use crate::game::module::GameStateView;
use crate::game::state::GameState;
use crate::markdown::BulletList;
use crate::util::prompting::{PromptFormatter, PromptifyError, PromptifyResult};

/// Persona of the one-shot summarizer model used during memory compression.
pub const SUMMARIZER_PERSONA: &str = r#"
You maintain the long-term memory of an interactive fiction session. You will
be given the current memory summary followed by a transcript of recent turns.
Merge the transcript into the summary: keep it a concise, factual narrative of
everything important that happened so far - characters met, key events,
promises made, unresolved threads, and the current situation. Preserve names
and details exactly. Omit filler and small talk. Output only the updated
summary.
"#;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemoryModule {}

impl MemoryModule {
  /// Overwrite the rolling story summary. Engine-internal; agents manage
  /// chapters via tools instead.
  pub(crate) async fn write_summary(state: &GameState, summary: String) -> Result<(), EngineError> {
    state.view(Self::id())
      .set(&Keypath::Summary.to_keypath(), &summary)
      .await
  }

  /// Read the current memory state. Missing keys default to empty.
  pub async fn read(state: &GameState) -> Result<MemoryState, EngineError> {
    let Some(module) = state.get(&Self::id()) else {
      return Ok(Default::default());
    };
    let summary = module.get::<String>(&Keypath::Summary.to_keypath()).await.transpose()?.unwrap_or_default();
    let chapters = module.get::<Vec<ChapterV2>>(&Keypath::Chapters.to_keypath()).await.transpose()?.unwrap_or_default();
    Ok(MemoryState { summary, chapters })
  }

  /// Read the chapters, bootstrapping an empty stub for missing or empty
  /// state: there is always an active (last) chapter. The active chapter is
  /// untitled until the agent closes it via `begin_chapter`.
  async fn read_chapters(ctx: &GameStateView) -> Result<Vec<ChapterV2>, EngineError> {
    let mut chapters = ctx.get::<Vec<ChapterV2>>(&Self::id(), &Keypath::Chapters.to_keypath())
      .await
      .transpose()?
      .unwrap_or_default();
    if chapters.is_empty() {
      chapters.push(ChapterV2::default());
    }
    Ok(chapters)
  }

  /// Render the agent-facing context: the rolling story summary and the
  /// chapter history. The story premise is NOT part of this — the engine
  /// injects it from the session's exposition into the system prompt.
  pub async fn context(&self, state: GameState, f: &mut PromptFormatter) -> PromptifyResult {
    let memory = Self::read(&state).await
      .map_err(|err| PromptifyError::generic(err.to_string()))?;

    if !memory.summary.trim().is_empty() {
      f.write("Story so far:")?;
      f.indented(|f| f.write(&memory.summary))?;
    }

    if !memory.chapters.is_empty() {
      f.write("Chapters:")?;
      f.indented(|f| {
        for chapter in &memory.chapters {
          if chapter.title.trim().is_empty() {
            f.writeline("(current chapter)")?;
          } else {
            f.writeline(&chapter.title)?;
          }
          f.indented(|f| f.write(chapter.notes.trim()))?;
        }
        Ok(())
      })?;
    }

    Ok(())
  }
}

#[toolset(strip_suffix = "module")]
impl AgentToolset<GameStateView> for MemoryModule {
  /// Begin a new chapter, closing the current one under the given title.
  /// Use this when the story reaches a significant shift in plot, location
  /// or cast, and give the closing chapter an appropriate, descriptive
  /// title.
  /// @title A descriptive title for the chapter that just ended.
  async fn begin_chapter(&self, ctx: GameStateView, title: String) -> Result<String, anyhow::Error> {
    let title = title.trim().to_string();
    if title.is_empty() {
      return Err(EngineError::validation("chapter title must not be empty").into());
    }

    let mut chapters = Self::read_chapters(&ctx).await?;
    let Some(active) = chapters.last_mut() else {
      unreachable!("read_chapters always yields an active chapter")
    };
    active.title = title;
    chapters.push(ChapterV2::default());

    ctx.set(&Keypath::Chapters.to_keypath(), &chapters).await?;
    Ok("Began a new chapter.".to_string())
  }

  /// Replace the event notes of the current chapter with the given concise
  /// bullet points. Note down important events as they happen.
  /// @notes The complete notes of the current chapter, one bullet point per
  /// event, each line starting with "- ".
  async fn note_events(&self, ctx: GameStateView, notes: String) -> Result<String, anyhow::Error> {
    let list = BulletList::parse(&notes)?;

    let mut chapters = Self::read_chapters(&ctx).await?;
    let Some(last) = chapters.last_mut() else {
      unreachable!("read_chapters always yields an active chapter")
    };
    last.notes = list.to_string();

    ctx.set(&Keypath::Chapters.to_keypath(), &chapters).await?;
    Ok("Chapter notes updated.".to_string())
  }
}

impl From<MemoryModule> for super::GameplayModule {
  fn from(value: MemoryModule) -> Self {
    Self::Memory(value)
  }
}

/// Read-only snapshot of the memory module state, for context building & GUI.
/// The story premise lives on the session (`SessionV2::exposition`), not in
/// gamestate.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct MemoryState {
  pub summary: String,
  pub chapters: Vec<ChapterV2>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ChapterV2 {
  /// Title of this chapter.
  pub title: String,
  /// Bullet-point record of important events of this chapter, as a markdown
  /// bullet list.
  pub notes: String,
}

enum Keypath {
  Summary,
  Chapters,
}

impl Keypath {
  fn to_keypath(&self) -> Vec<String> {
    match self {
      Self::Summary  => vec!["summary".to_string()],
      Self::Chapters => vec!["chapters".to_string()],
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn state() -> GameState {
    GameState::new(Default::default(), vec![MemoryModule::ID.to_string()], None).unwrap()
  }

  #[tokio::test]
  async fn read_defaults_empty() {
    let memory = MemoryModule::read(&state()).await.unwrap();
    assert_eq!(memory, MemoryState::default());
  }

  #[tokio::test]
  async fn chapters_bootstrap_on_first_tool_use() {
    let state = state();
    let module = MemoryModule::default();

    module.note_events(state.view(MemoryModule::ID.to_string()), "- a lone traveler arrives".to_string()).await.unwrap();

    let memory = MemoryModule::read(&state).await.unwrap();
    assert_eq!(memory.chapters.len(), 1);
    assert!(memory.chapters[0].title.is_empty());
    assert_eq!(memory.chapters[0].notes, "- a lone traveler arrives\n");
  }

  #[tokio::test]
  async fn begin_chapter_titles_the_closing_chapter() {
    let state = state();
    let module = MemoryModule::default();

    // Chapter 1 happens, then gets titled as it closes; each begin_chapter
    // opens the next untitled chapter.
    module.note_events(state.view(MemoryModule::ID.to_string()), "- met a merchant\n- bought a horse".to_string()).await.unwrap();
    module.begin_chapter(state.view(MemoryModule::ID.to_string()), "The Long Road".to_string()).await.unwrap();
    module.note_events(state.view(MemoryModule::ID.to_string()), "- arrived at the gates".to_string()).await.unwrap();
    module.begin_chapter(state.view(MemoryModule::ID.to_string()), "The City".to_string()).await.unwrap();

    let memory = MemoryModule::read(&state).await.unwrap();
    assert_eq!(memory.chapters.len(), 3);
    assert_eq!(memory.chapters[0].title, "The Long Road");
    assert_eq!(memory.chapters[0].notes, "- met a merchant\n- bought a horse\n");
    assert_eq!(memory.chapters[1].title, "The City");
    assert_eq!(memory.chapters[1].notes, "- arrived at the gates\n");
    // The newly opened chapter remains an untitled, noteless stub.
    assert_eq!(memory.chapters[2], ChapterV2::default());
  }

  #[tokio::test]
  async fn begin_chapter_rejects_empty_title() {
    let state = state();
    let module = MemoryModule::default();

    assert!(module.begin_chapter(state.view(MemoryModule::ID.to_string()), String::new()).await.is_err());
    assert!(module.begin_chapter(state.view(MemoryModule::ID.to_string()), "   ".to_string()).await.is_err());
    // Nothing was persisted — not even a bootstrapped chapter.
    let memory = MemoryModule::read(&state).await.unwrap();
    assert!(memory.chapters.is_empty());
  }

  #[tokio::test]
  async fn note_events_rejects_non_bullets() {
    let state = state();
    let module = MemoryModule::default();
    assert!(module.note_events(state.view(MemoryModule::ID.to_string()), "plain prose".to_string()).await.is_err());
    // Even after a chapter exists:
    module.begin_chapter(state.view(MemoryModule::ID.to_string()), "T".to_string()).await.unwrap();
    assert!(module.note_events(state.view(MemoryModule::ID.to_string()), "plain prose".to_string()).await.is_err());
  }
}
