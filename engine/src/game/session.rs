use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use kiruklaw_agent_loop::{Conversation, ConversationMessage, ModelConfig, prompt as llm_prompt};
use serde::{Deserialize, Deserializer, Serialize};
use uuid::Uuid;

use crate::error::EngineError;
use crate::game::module::GameplayModule;
use crate::game::module::story::{StoryModule, SUMMARIZER_PERSONA};
use crate::game::profile::ProfileV1;
use crate::game::state::GameState;
use crate::game::state::Snapshot;
use crate::game::turns::TurnV2;
use crate::util::{deserialize, deserialize_timestamp, discard_channel, serialize, serialize_timestamp};
use crate::util::prompting::{PromptFormatter, PromptifyResult};

/// The complete persisted state of a session, stored as a single file
/// (`{sessionsDir}/{id}.json`): session metadata, the gamestate snapshot
/// anchoring the retained history, and the retained short-term turns.
/// One file is the single source of truth for restoring a session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionV2 {
  #[serde(deserialize_with = "SessionV2::deserialize_version")]
  pub version: u8,
  pub id: String,
  pub title: String,
  /// Brief story exposition serving as the starting point of the story.
  pub exposition: String,
  #[serde(serialize_with = "serialize_timestamp", deserialize_with = "deserialize_timestamp")]
  #[serde(rename = "created_at")]
  pub created_at: DateTime<Utc>,
  #[serde(serialize_with = "serialize_timestamp", deserialize_with = "deserialize_timestamp")]
  #[serde(rename = "updated_at")]
  pub updated_at: DateTime<Utc>,
  /// Configured gameplay modules that this session uses.
  pub modules: HashMap<String, GameplayModule>,
  /// Optional profile ID that was last used for this session.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub profile: Option<String>,
  /// Total number of turns ever pushed, including turns that were since
  /// compressed away into the rolling summary.
  pub total_turns: u64,
  /// Gamestate as it was just before the first retained turn. Re-anchored
  /// whenever turns are compressed away.
  pub snapshot: Snapshot,
  /// Retained short-term memory: the last N turns of the conversation.
  pub turns: Vec<TurnV2>,
  #[serde(skip)]
  pub(crate) gamestate: GameState,
}

impl Default for SessionV2 {
  fn default() -> Self {
    let id = Uuid::new_v4().to_string();
    let modules = GameplayModule::default_set();
    let gamestate = GameState::new(Default::default(), modules.keys().cloned(), None)
      .expect("fresh module states are always valid");
    Self {
      version: Self::VERSION,
      id: id.clone(),
      title: String::new(),
      exposition: String::new(),
      created_at: Utc::now(),
      updated_at: Utc::now(),
      modules,
      profile: None,
      total_turns: 0,
      snapshot: Snapshot::default(),
      turns: vec![],
      gamestate,
    }
  }
}

impl SessionV2 {
  pub const VERSION: u8 = 2u8;

  #[inline(always)]
  pub fn set_profile(&mut self, profile: Option<Arc<ProfileV1>>) {
    self.profile = profile.as_ref().map(|p| p.id.clone());
    self.gamestate.set_profile(profile);
  }

  #[inline(always)]
  pub fn root() -> Result<PathBuf, EngineError> {
    crate::paths::sessions_dir()
  }

  #[inline(always)]
  pub fn path(id: &str) -> Result<PathBuf, EngineError> {
    Ok(Self::root()?.join(format!("{id}.json")))
  }

  /// Restore the given session by `id` from the local filesystem,
  /// rebuilding the gamestate from the snapshot + replay of the retained
  /// turns. Sessions with an unexpected file version are rejected.
  pub async fn load(id: impl Into<String>, profiles: &[ProfileV1]) -> Result<SessionV2, EngineError> {
    let path = Self::path(&id.into())?;
    let mut res: SessionV2 = deserialize(&path).await?;
    res.rebuild_gamestate(profiles).await?;
    Ok(res)
  }

  /// Rebuild the in-memory gamestate from the session snapshot + replay of
  /// the retained turns.
  pub(crate) async fn rebuild_gamestate(&mut self, profiles: &[ProfileV1]) -> Result<(), EngineError> {
    let profile_id = self.profile.clone().unwrap_or_default();
    let profile = profiles.iter()
      .find(|p| p.id == profile_id)
      .map(|p| Arc::new(p.clone()));
    self.gamestate = GameState::new(self.snapshot.clone(), self.modules.keys().cloned(), profile)?;
    self.gamestate.replay(&self.modules, &self.turns).await?;
    Ok(())
  }

  /// Save this session to disk as a single file.
  pub async fn save(&self) -> Result<(), EngineError> {
    serialize(&Self::path(&self.id)?, self).await
  }

  /// Load a session file without rebuilding its game state. The returned
  /// session only carries persisted data (metadata, snapshot, turns).
  pub async fn load_metadata(id: impl Into<String>) -> Result<SessionV2, EngineError> {
    let path = Self::path(&id.into())?;
    deserialize(&path).await
  }

  /// Delete the session file with the given `id` from the local filesystem.
  /// Deleting a nonexistent session is a no-op.
  pub async fn delete(id: &str) -> Result<(), EngineError> {
    let path = Self::path(id)?;
    if !path.exists() {
      return Ok(());
    }
    tokio::fs::remove_file(&path).await?;
    Ok(())
  }

  /// Number of turns in the retained short-term history.
  #[inline(always)]
  pub fn turn_count(&self) -> usize {
    self.turns.len()
  }

  /// The stored rolling story summary from the memory module's snapshot
  /// state, or empty if there is none. Reads the persisted snapshot
  /// directly — no gamestate rebuild — so it also works on metadata-only
  /// loads (as used by the sessions list).
  pub fn summary(&self) -> String {
    self.snapshot
      .get(StoryModule::ID)
      .and_then(|state| state.get("summary"))
      .and_then(|value| value.as_str())
      .unwrap_or_default()
      .to_string()
  }

  /// Get a retained turn by its window-relative index, if any.
  pub fn try_turn(&self, index: usize) -> Option<&TurnV2> {
    self.turns.get(index)
  }

  /// Get an iterator over the module IDs of this session
  pub fn modules(&self) -> impl Iterator<Item = &String> {
    self.modules.keys()
  }

  /// Get a module by its ID, if any
  pub fn module(&self, module: &String) -> Option<&GameplayModule> {
    self.modules.get(module)
  }

  /// Push a new turn to the end of the session's history.
  pub async fn push_turn(&mut self, turn: TurnV2) -> Result<(), EngineError> {
    self.updated_at = Utc::now();
    self.turns.push(turn);
    self.total_turns += 1;
    self.save().await
  }

  /// Update the last turn of the session.
  pub async fn update_turn(&mut self, cb: impl FnOnce(TurnV2) -> Result<TurnV2, EngineError>) -> Result<(), EngineError> {
    let Some(turn) = self.turns.pop() else {
      return Err(EngineError::state("fresh session"));
    };
    self.turns.push(cb(turn)?);
    self.save().await
  }

  /// Replace the last turn of the session with the given turn.
  pub async fn replace_turn(&mut self, turn: TurnV2) -> Result<(), EngineError> {
    self.turns.pop();
    self.turns.push(turn);
    self.save().await
  }

  /// Drop the last turn of the session.
  pub async fn pop_turn(&mut self) -> Result<Option<TurnV2>, EngineError> {
    self.updated_at = Utc::now();
    let turn = self.turns.pop();
    self.save().await?;
    Ok(turn)
  }

  /// Fork this session from the given retained turn index, deleting all
  /// later turns. The gamestate is rebuilt from the session snapshot plus a
  /// replay of the retained turns. Only turns within the retained window can
  /// be forked from.
  pub async fn fork(&mut self, turn_index: usize, profiles: &[ProfileV1]) -> Result<(), EngineError> {
    if turn_index >= self.turn_count() {
      return Err(EngineError::illegal(format!(
        "cannot fork at turn {turn_index}: outside the retained history of {} turns",
        self.turn_count()
      )));
    }

    self.updated_at = Utc::now();
    self.turns.truncate(turn_index + 1);
    self.rebuild_gamestate(profiles).await?;
    self.save().await
  }

  /// Drop all retained turns, restoring the gamestate to the session
  /// snapshot. Unlike [Self::fork] this may empty the history entirely —
  /// used when regenerating the very first turn.
  pub async fn reset_turns(&mut self, profiles: &[ProfileV1]) -> Result<(), EngineError> {
    self.updated_at = Utc::now();
    self.turns.clear();
    self.rebuild_gamestate(profiles).await?;
    self.save().await
  }

  /// Whether the retained history exceeds `2 * threshold` turns and should
  /// be compressed.
  #[inline(always)]
  pub fn should_compress(&self, threshold: usize) -> bool {
    threshold > 0 && self.turns.len() > threshold * 2
  }

  /// Compress the oldest `threshold` turns into the rolling story summary
  /// once the retained history exceeds `2 * threshold` turns. The gamestate
  /// snapshot is re-anchored to the new first retained turn, so rolling back
  /// remains possible for everything within the retained window.
  ///
  /// Returns whether a compression happened. If the summarizer model fails,
  /// nothing is mutated and the compression is simply retried on a later
  /// trigger.
  pub async fn compress_memory(&mut self, threshold: usize, model: &ModelConfig) -> Result<bool, EngineError> {
    if !self.should_compress(threshold) {
      return Ok(false);
    }

    let previous = StoryModule::read(&self.gamestate).await?.summary;
    let transcript = {
      let mut f = PromptFormatter::new();
      f.indented(|f| render_transcript(f, &self.turns[..threshold], &previous))?;
      f.finish()
    };

    // Summarize BEFORE mutating anything, so a failed LLM call leaves the
    // session untouched.
    let summary = summarize(model, &transcript).await?;

    self.turns.drain(..threshold);
    StoryModule::write_summary(&self.gamestate, summary).await?;
    self.snapshot = self.gamestate.snapshot().await;
    self.save().await?;
    Ok(true)
  }

  pub(crate) fn conversation(&self) -> Conversation {
    TurnV2::to_conversation(&self.turns)
  }

  fn deserialize_version<'de, D: Deserializer<'de>>(deserializer: D) -> Result<u8, D::Error> {
    let version = u8::deserialize(deserializer)?;
    if version != Self::VERSION {
      return Err(serde::de::Error::custom(format!("Unexpected version {}, expected {}", version, Self::VERSION)));
    }
    Ok(version)
  }
}

/// Render the given turns into a text transcript for the summarizer model,
/// prefixed by the previous rolling summary, if any.
fn render_transcript(f: &mut PromptFormatter, turns: &[TurnV2], prev: &str) -> PromptifyResult {
  if !prev.is_empty() {
    f.write("Previous summary: ")?;
    f.writeline(prev)?;
    f.newline()?;
  }

  for turn in turns.iter() {
    if let Some(prompt) = &turn.prompt {
      f.write("Player: ")?;
      f.writeline(prompt)?;
      f.newline()?;
    }
    for response in &turn.responses {
      if let Some(content) = &response.content {
        f.write("DM: ")?;
        f.writeline(content)?;
        f.newline()?;
      }
      if !response.tool_calls.is_empty() {
        f.write("DM actions: ")?;
        for (i, tc) in response.tool_calls.iter().enumerate() {
          f.write(&tc.name)?;
          if i < response.tool_calls.len() - 1 {
            f.write(", ")?;
          }
        }
        f.newline()?;
        f.newline()?;
      }
    }
  }
  Ok(())
}

/// One-shot summarizer call merging a turn transcript (including the
/// previous rolling summary) into the new rolling story summary.
async fn summarize(model: &ModelConfig, transcript: &str) -> Result<String, EngineError> {
  let (msg, _) = llm_prompt(
    model,
    &[],
    vec![
      ConversationMessage::system(SUMMARIZER_PERSONA.to_string()).into(),
      ConversationMessage::user(transcript.to_string()).into(),
    ],
    // The summarizer is a one-shot call — no reasoning requested.
    None,
    discard_channel(),
  ).await?;

  let ConversationMessage::Assistant { content, .. } = msg else { unreachable!() };
  Ok(content.trim().to_string())
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn fresh_session_seeds_module_state() {
    let session = SessionV2::default();
    for modid in session.modules.keys() {
      assert!(session.gamestate.get(modid).is_some(), "module {modid} not seeded");
    }
    assert_eq!(session.turn_count(), 0);
    assert_eq!(session.total_turns, 0);
    assert!(!session.should_compress(50));
  }

  #[test]
  fn session_file_roundtrip() {
    let mut session = SessionV2::default();
    session.title = "The Test".to_string();
    session.exposition = "A premise.".to_string();
    session.total_turns = 7;
    session.snapshot.insert("memory".to_string(), serde_json::json!({"summary": "so far"}));
    session.turns = vec![TurnV2 {
      prompt: Some("look around".to_string()),
      ..Default::default()
    }];

    let json = serde_json::to_string(&session).unwrap();
    let restored: SessionV2 = serde_json::from_str(&json).unwrap();

    assert_eq!(restored.version, SessionV2::VERSION);
    assert_eq!(restored.id, session.id);
    assert_eq!(restored.title, "The Test");
    assert_eq!(restored.exposition, "A premise.");
    assert_eq!(restored.total_turns, 7);
    assert_eq!(restored.snapshot, session.snapshot);
    // TurnV2 contains ToolCalls, which carry no PartialEq — compare serialized.
    assert_eq!(
      serde_json::to_string(&restored.turns).unwrap(),
      serde_json::to_string(&session.turns).unwrap(),
    );
  }

  #[test]
  fn session_rejects_wrong_version() {
    let json = r#"{"version": 1, "id": "x"}"#;
    assert!(serde_json::from_str::<SessionV2>(json).is_err());
  }

  #[test]
  fn summary_reads_memory_snapshot() {
    let mut session = SessionV2::default();
    assert_eq!(session.summary(), "");
    session.snapshot.insert(
      StoryModule::ID.to_string(),
      serde_json::json!({"summary": "so far", "chapters": []}),
    );
    assert_eq!(session.summary(), "so far");
  }

  #[test]
  fn compression_trigger_thresholds() {
    let mut session = SessionV2::default();
    session.turns = vec![TurnV2::default(); 100];
    assert!(!session.should_compress(0));
    assert!(!session.should_compress(50)); // exactly 2x — not over
    session.turns.push(TurnV2::default());
    assert!(session.should_compress(50)); // 101 > 100
    session.turns.truncate(99);
    assert!(session.should_compress(49));
  }

  #[tokio::test]
  async fn fork_truncates_and_keeps_anchor() {
    let mut session = SessionV2::default();
    session.turns = (0..10).map(|_| TurnV2::default()).collect();
    session.total_turns = 10;
    session.snapshot = gamestate_snapshot(&session).await;

    session.fork(4, &[]).await.unwrap();
    assert_eq!(session.turn_count(), 5);
    assert_eq!(session.total_turns, 10);
  }

  #[tokio::test]
  async fn fork_rejects_out_of_window_index() {
    let mut session = SessionV2::default();
    session.turns = vec![TurnV2::default(); 3];
    assert!(session.fork(3, &[]).await.is_err());
    assert!(session.fork(100, &[]).await.is_err());
    assert_eq!(session.turn_count(), 3);
  }

  #[tokio::test]
  async fn fork_rebuilds_gamestate_from_replay() {
    let mut session = SessionV2::default();
    session.snapshot = gamestate_snapshot(&session).await;
    // A tool call recorded on turn 0 must be re-applied on rebuild.
    session.turns = vec![TurnV2 {
      prompt: Some("begin".to_string()),
      responses: vec![super::super::turns::AgentResponse {
        tool_calls: vec![kiruklaw_agent_loop::ToolCall {
          id: "tc1".to_string(),
          name: "story::begin_chapter".to_string(),
          arguments: r#"{"title":"Exodus"}"#.to_string(),
        }],
        tool_responses: vec![("tc1".to_string(), "Began a new chapter.".to_string())],
        ..Default::default()
      }],
      ..Default::default()
    }];
    session.total_turns = 1;

    session.rebuild_gamestate(&[]).await.unwrap();
    let story = StoryModule::read(&session.gamestate).await.unwrap();
    // The replayed call titled the bootstrapped first chapter and opened a
    // fresh stub.
    assert_eq!(story.chapters.len(), 2);
    assert_eq!(story.chapters[0].title, "Exodus");
    assert!(story.chapters[1].title.is_empty());
  }

  #[test]
  fn conversation_from_turns() {
    let mut session = SessionV2::default();
    session.turns = vec![TurnV2 {
      prompt: Some("hello".to_string()),
      responses: vec![super::super::turns::AgentResponse {
        content: Some("hi there".to_string()),
        ..Default::default()
      }],
      ..Default::default()
    }];

    let conv = session.conversation();
    assert_eq!(conv.messages.len(), 2);
    assert!(matches!(&conv.messages[0], ConversationMessage::User { content } if content == "hello"));
    assert!(matches!(&conv.messages[1], ConversationMessage::Assistant { content, .. } if content == "hi there"));
  }

  #[test]
  fn transcript_renders_prev_summary_and_parts() {
    let turns = vec![
      TurnV2 {
        prompt: Some("hello".to_string()),
        responses: vec![super::super::turns::AgentResponse {
          content: Some("hi there".to_string()),
          tool_calls: vec![kiruklaw_agent_loop::ToolCall {
            id: "t".to_string(),
            name: "story::note_events".to_string(),
            arguments: "{}".to_string(),
          }],
          ..Default::default()
        }],
        ..Default::default()
      },
    ];
    let mut f = PromptFormatter::new();
    render_transcript(&mut f, &turns, "Once upon a time.").unwrap();
    let text = f.finish();
    assert!(text.contains("Previous summary: Once upon a time."));
    assert!(text.contains("Player: hello"));
    assert!(text.contains("DM: hi there"));
    assert!(text.contains("story::note_events"));

    // An empty previous summary is omitted entirely.
    let mut f = PromptFormatter::new();
    render_transcript(&mut f, &turns, &String::new()).unwrap();
    assert!(!f.finish().contains("Previous summary"));
  }

  async fn gamestate_snapshot(session: &SessionV2) -> Snapshot {
    session.gamestate.snapshot().await
  }
}
