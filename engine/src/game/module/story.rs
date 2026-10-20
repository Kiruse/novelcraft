use kiruklaw_agent_loop::{AgentToolset, toolset};
use serde::{Deserialize, Serialize};

use crate::error::EngineError;
use crate::game::module::GameStateView;
use crate::game::state::GameState;
use crate::markdown::{BulletList, TodoList};
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
pub struct StoryModule {}

impl StoryModule {
  /// Overwrite the rolling story summary. Engine-internal; agents manage
  /// chapters via tools instead.
  pub(crate) async fn write_summary(state: &GameState, summary: String) -> Result<(), EngineError> {
    state.view(Self::id())
      .set(&Keypath::Summary.to_keypath(), &summary)
      .await
  }

  /// Read the long-term story state. Missing keys default to empty.
  pub async fn read(state: &GameState) -> Result<StoryState, EngineError> {
    let Some(module) = state.get(&Self::id()) else {
      return Ok(Default::default());
    };
    let summary = module.get::<String>(&Keypath::Summary.to_keypath()).await.transpose()?.unwrap_or_default();
    let chapters = module.get::<Vec<ChapterV2>>(&Keypath::Chapters.to_keypath()).await.transpose()?.unwrap_or_default();
    Ok(StoryState { summary, chapters })
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

  /// Render the agent-facing context: the timeline, the rolling story
  /// summary and the chapter history. The story premise is NOT part of
  /// this — the engine injects it from the session's exposition into the
  /// system prompt.
  pub async fn context(&self, state: GameState, f: &mut PromptFormatter) -> PromptifyResult {
    let Some(module) = state.get(&Self::id()) else { return Ok(()) };
    let timeline = module.get::<String>(&Keypath::Timeline.to_keypath())
      .await
      .transpose()
      .map_err(|err| PromptifyError::generic(err.to_string()))?
      .unwrap_or_default()
      .trim()
      .to_string();
    let story = Self::read(&state).await
      .map_err(|err| PromptifyError::generic(err.to_string()))?;

    if !timeline.is_empty() {
      f.write("## Timeline")?;
      f.indented(|f| f.write(&timeline))?;
    }

    if !story.summary.trim().is_empty() {
      f.write("Story so far:")?;
      f.indented(|f| f.write(&story.summary))?;
    }

    if !story.chapters.is_empty() {
      f.write("## Chapters")?;
      f.indented(|f| {
        for chapter in &story.chapters {
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
impl AgentToolset<GameStateView> for StoryModule {
  /// Manage the active timeline in the form of a TODO list of markdown task items, one per line:
  /// `- [ ] outstanding item` or `- [x] visited item`. Checked-off items have been visited,
  /// unchecked items are outstanding. The timeline is merged, not replaced: new items are appended
  /// at the end and unchecked items become checked once you mark them visited. Checked items can
  /// never be unchecked or removed — attempts to do so are silently ignored.
  /// @contents The new contents of the timeline (items to add or check off).
  async fn manage_timeline(&self, ctx: GameStateView, contents: String) -> Result<String, anyhow::Error> {
    let keypath = Keypath::Timeline.to_keypath();
    let existing = ctx.get::<String>(&Self::id(), &keypath)
      .await
      .transpose()?
      .unwrap_or_default();
    let existing = TodoList::parse(&existing)?;
    let new      = TodoList::parse(&contents)?;

    let merged = existing.merge(&new);
    ctx.set(&keypath, &merged.to_string()).await?;
    Ok("Timeline updated.".to_string())
  }

  /// Close the current chapter under the given title and begin a new,
  /// empty one. Only call this once the current chapter actually contains
  /// recorded events — never at the very start of the story, and never to
  /// pre-name a future chapter. A good moment is a significant shift in
  /// plot, location or cast; give the closing chapter an appropriate,
  /// descriptive title.
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

  /// Append event bullets to the current chapter's notes. Note down
  /// important events as they happen, one bullet point per event, each line
  /// starting with "- ". The notes merge with the existing record: bullets
  /// already noted are ignored, new ones are appended at the end — the
  /// record never shrinks, so you may re-submit earlier notes safely.
  /// @notes The event bullets to record.
  async fn note_events(&self, ctx: GameStateView, notes: String) -> Result<String, anyhow::Error> {
    let new_list = BulletList::parse(&notes)?;

    let mut chapters = Self::read_chapters(&ctx).await?;
    let Some(last) = chapters.last_mut() else {
      unreachable!("read_chapters always yields an active chapter")
    };
    let existing = BulletList::parse(&last.notes)?;
    last.notes = existing.merge(&new_list).to_string();

    ctx.set(&Keypath::Chapters.to_keypath(), &chapters).await?;
    Ok("Chapter notes updated.".to_string())
  }
}

impl From<StoryModule> for super::GameplayModule {
  fn from(value: StoryModule) -> Self {
    Self::Story(value)
  }
}

/// Read-only snapshot of the story module's long-term state, for context
/// building & GUI. The story premise lives on the session
/// (`SessionV2::exposition`), not in gamestate.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct StoryState {
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
  Timeline,
  Summary,
  Chapters,
}

impl Keypath {
  fn to_keypath(&self) -> Vec<String> {
    match self {
      Self::Timeline => vec!["timeline".to_string()],
      Self::Summary  => vec!["summary".to_string()],
      Self::Chapters => vec!["chapters".to_string()],
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn state() -> GameState {
    GameState::new(Default::default(), vec![StoryModule::ID.to_string()], None).unwrap()
  }

  #[tokio::test]
  async fn manage_timeline_roundtrip() {
    let state = state();
    let module = StoryModule::default();
    let view = state.view(StoryModule::ID.to_string());

    module.manage_timeline(view, "[x] intro\n[ ] the city".to_string()).await.unwrap();
    let mut f = PromptFormatter::new();
    module.context(state, &mut f).await.unwrap();
    assert!(f.finish().contains("## Timeline"));
  }

  #[tokio::test]
  async fn manage_timeline_merges_instead_of_replacing() {
    let module = StoryModule::default();
    let view = state().view(StoryModule::ID.to_string());

    module.manage_timeline(view.clone(), "[ ] intro\n[x] the road".to_string()).await.unwrap();
    // Checks off `intro`, appends `the city` at the end; `the road` stays.
    module.manage_timeline(view.clone(), "[x] intro\n[ ] the city".to_string()).await.unwrap();
    // Uncheck (`intro`) and remove (`the road`) attempts are ignored.
    module.manage_timeline(view.clone(), "[ ] intro".to_string()).await.unwrap();

    let timeline = view.get::<String>(&StoryModule::id().to_string(), &Keypath::Timeline.to_keypath())
      .await
      .transpose()
      .unwrap()
      .unwrap();
    assert_eq!(timeline, "- [x] intro\n- [x] the road\n- [ ] the city");
  }

  #[tokio::test]
  async fn read_defaults_empty() {
    let story = StoryModule::read(&state()).await.unwrap();
    assert_eq!(story, StoryState::default());
  }

  #[tokio::test]
  async fn chapters_bootstrap_on_first_tool_use() {
    let state = state();
    let module = StoryModule::default();

    module.note_events(state.view(StoryModule::ID.to_string()), "- a lone traveler arrives".to_string()).await.unwrap();

    let story = StoryModule::read(&state).await.unwrap();
    assert_eq!(story.chapters.len(), 1);
    assert!(story.chapters[0].title.is_empty());
    assert_eq!(story.chapters[0].notes, "- a lone traveler arrives\n");
  }

  #[tokio::test]
  async fn note_events_appends_and_dedupes() {
    let state = state();
    let module = StoryModule::default();
    let view = state.view(StoryModule::ID.to_string());

    module.note_events(view.clone(), "- a traveler arrives".to_string()).await.unwrap();
    // Next call records only the new events — earlier notes survive.
    module.note_events(view.clone(), "- meets the merchant".to_string()).await.unwrap();
    // Re-submitting the full list is safe: duplicates are ignored.
    module.note_events(view.clone(), "- a traveler arrives\n- meets the merchant\n- buys a horse".to_string()).await.unwrap();

    let story = StoryModule::read(&state).await.unwrap();
    assert_eq!(
      story.chapters[0].notes,
      "- a traveler arrives\n- meets the merchant\n- buys a horse\n"
    );
  }

  #[tokio::test]
  async fn begin_chapter_titles_the_closing_chapter() {
    let state = state();
    let module = StoryModule::default();

    // Chapter 1 happens, then gets titled as it closes; each begin_chapter
    // opens the next untitled chapter.
    module.note_events(state.view(StoryModule::ID.to_string()), "- met a merchant\n- bought a horse".to_string()).await.unwrap();
    module.begin_chapter(state.view(StoryModule::ID.to_string()), "The Long Road".to_string()).await.unwrap();
    module.note_events(state.view(StoryModule::ID.to_string()), "- arrived at the gates".to_string()).await.unwrap();
    module.begin_chapter(state.view(StoryModule::ID.to_string()), "The City".to_string()).await.unwrap();

    let story = StoryModule::read(&state).await.unwrap();
    assert_eq!(story.chapters.len(), 3);
    assert_eq!(story.chapters[0].title, "The Long Road");
    assert_eq!(story.chapters[0].notes, "- met a merchant\n- bought a horse\n");
    assert_eq!(story.chapters[1].title, "The City");
    assert_eq!(story.chapters[1].notes, "- arrived at the gates\n");
    // The newly opened chapter remains an untitled, noteless stub.
    assert_eq!(story.chapters[2], ChapterV2::default());
  }

  #[tokio::test]
  async fn begin_chapter_rejects_empty_title() {
    let state = state();
    let module = StoryModule::default();

    assert!(module.begin_chapter(state.view(StoryModule::ID.to_string()), String::new()).await.is_err());
    assert!(module.begin_chapter(state.view(StoryModule::ID.to_string()), "   ".to_string()).await.is_err());
    // Nothing was persisted — not even a bootstrapped chapter.
    let story = StoryModule::read(&state).await.unwrap();
    assert!(story.chapters.is_empty());
  }

  #[tokio::test]
  async fn note_events_rejects_non_bullets() {
    let state = state();
    let module = StoryModule::default();
    assert!(module.note_events(state.view(StoryModule::ID.to_string()), "plain prose".to_string()).await.is_err());
    // Even after a chapter exists:
    module.begin_chapter(state.view(StoryModule::ID.to_string()), "T".to_string()).await.unwrap();
    assert!(module.note_events(state.view(StoryModule::ID.to_string()), "plain prose".to_string()).await.is_err());
  }
}
