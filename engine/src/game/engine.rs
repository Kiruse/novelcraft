use std::collections::HashMap;
use std::sync::Arc;

use kiruklaw_agent_loop::{AgentLoop, AgentMessageChunk, ContextProvider, Conversation, ConversationMessage};
use kiruklaw_agent_loop::openai::OpenAiReasoningConfig;
use log::{info, warn};
use tokio::sync::mpsc::Sender;
use uuid::Uuid;

use crate::config::{ModelPurpose, NovelCraftConfig};
use crate::error::EngineError;
use crate::game::module::story::{StoryModule, StoryState};
use crate::game::profile::ProfileV1;
use crate::game::session::SessionV2;
use crate::game::state::{GameState, GameStateView};
use crate::game::turns::TurnV2;
use crate::util::prompting::{PromptFormatter, Promptify};

/// Per-turn system message injected immediately before the latest user
/// prompt, nudging the DM agent to document the turn via its tools. This is
/// the project's USP — relevant events must not go unrecorded just because
/// the persona prompt is far back in the context.
const TOOL_REMINDER: &str = "# Turn reminder
DO NOT FORGET to track changes to the story through your tools!
BE SURE to manage your NPCs and keep track of plot events in the chapters and timeline.
BE EAGER with your tool calls! We rather store too much data than too little.
The moment you assign an NPC a name, consider them important enough to track.";

pub struct NovelCraftEngine {
  /// NovelCraft game engine config
  config: NovelCraftConfig,
  /// Active user session
  session: Option<SessionV2>,
  /// Cache of module context prompt component strings.
  module_context_cache: HashMap<String, String>,
  /// Cached AgentLoop config
  agent_loop: Option<AgentLoop<GameStateView>>,
}

impl NovelCraftEngine {
  pub fn new(config: NovelCraftConfig) -> Self {
    Self {
      config,
      session: None,
      module_context_cache: HashMap::new(),
      agent_loop: None,
    }
  }

  pub fn set_config(&mut self, config: NovelCraftConfig) {
    config.contribute(self.agent_loop.as_mut());
    self.config = config;
  }

  pub fn set_active_profile(&mut self, profile_id: Option<String>) {
    self.config.active_profile = profile_id.clone();

    if let Some(session) = self.session.as_mut() {
      let profile = self.config.profiles
        .iter()
        .find(|p| Some(&p.id) == profile_id.as_ref())
        .cloned()
        .map(Arc::new);
      session.set_profile(profile);

      self.agent_loop = build_agent_loop(
        std::mem::take(&mut self.agent_loop),
        &self.config,
        Some(session),
      );
    }
  }

  /// Read-only access to the engine's active configuration.
  pub fn config(&self) -> &NovelCraftConfig {
    &self.config
  }

  /// Number of turns in the active session's retained history, if any.
  pub fn turn_count(&self) -> Option<usize> {
    Some(self.session.as_ref()?.turn_count())
  }

  pub async fn save(&self) -> Result<(), EngineError> {
    if let Some(session) = self.session.as_ref() {
      session.save().await?;
    }
    self.config.save().await?;
    Ok(())
  }

  pub fn set_session(&mut self, session: Option<SessionV2>) {
    self.session = session;
    self.agent_loop = build_agent_loop(
      std::mem::take(&mut self.agent_loop),
      &self.config,
      self.session.as_ref(),
    );
  }

  #[inline(always)]
  pub fn session(&self) -> Result<&SessionV2, EngineError> {
    self.session.as_ref().ok_or(EngineError::state("no active session"))
  }
  #[inline(always)]
  fn session_mut(&mut self) -> Result<&mut SessionV2, EngineError> {
    self.session.as_mut().ok_or(EngineError::state("no active session"))
  }

  #[inline(always)]
  fn gamestate(&self) -> Result<GameState, EngineError> {
    self.session().map(|s| s.gamestate.clone())
  }

  /// Enumerate all saved sessions on the local filesystem, newest first.
  /// Only persisted session data is read; game states are left untouched.
  /// Sessions with an unexpected file version (including legacy multi-file
  /// session directories) are skipped.
  pub async fn list_sessions() -> Result<Vec<SessionV2>, EngineError> {
    let path = SessionV2::root()?;
    crate::util::fs::ensure_dir(&path).await?;
    let mut result = Vec::new();
    for (name, is_dir) in crate::util::fs::list_dir(&path).await? {
      if is_dir {
        warn!("Ignoring legacy session directory {name:?}");
        continue;
      }
      let Some(sid) = name.strip_suffix(".json") else { continue };
      match SessionV2::load_metadata(sid).await {
        Ok(session) => result.push(session),
        Err(err) => warn!("Skipping session {sid}: {err}"),
      }
    }
    result.sort_by_key(|s| std::cmp::Reverse(s.updated_at));
    Ok(result)
  }

  /// Create and persist a new session with the given title & exposition.
  /// The session is set up with fresh instances of all existing gameplay
  /// modules and records the engine's active profile (if any).
  pub async fn create_session(
    &mut self,
    title: String,
    exposition: String,
  ) -> Result<SessionV2, EngineError> {
    let mut session = SessionV2 {
      title,
      exposition,
      ..Default::default()
    };
    session.set_profile(self.profile().map(|p| Arc::new(p.clone())));
    session.save().await?;
    Ok(session)
  }

  fn conversation(&self, modids: &[String]) -> Result<Conversation, EngineError> {
    let mut conv = Conversation::default();
    conv.push(self.system_prompt_msg(modids)?);
    conv.extend(self.session()?.conversation());
    inject_tool_reminder(&mut conv);
    Ok(conv)
  }

  /// Get a retained turn by its window-relative index, if any.
  pub fn turn(&self, index: usize) -> Option<TurnV2> {
    self.session.as_ref()?.try_turn(index).cloned()
  }

  /// Append a new profile (seeded with the default character fields), make
  /// it the active one, and persist. Returns the created profile.
  pub async fn create_profile(&mut self) -> Result<ProfileV1, EngineError> {
    let profile = ProfileV1::new(Uuid::new_v4().to_string())
      .with_name("New profile".to_string())
      .with_fields(vec![
        ("Gender".to_string(), String::new()),
        ("Age".to_string(), String::new()),
        ("Personality".to_string(), String::new()),
        ("Appearance".to_string(), String::new()),
      ]);
    self.config.profiles.push(profile.clone());
    self.set_active_profile(Some(profile.id.clone()));
    self.save().await?;
    Ok(profile)
  }

  /// Replace the given profile (matched by id) and persist. If it is the
  /// active profile, the active session's copy is refreshed too.
  pub async fn update_profile(&mut self, profile: ProfileV1) -> Result<(), EngineError> {
    let Some(existing) = self.config.profiles.iter_mut().find(|p| p.id == profile.id) else {
      return Err(EngineError::state(format!("no profile with id {}", profile.id)));
    };
    *existing = profile;
    let id = existing.id.clone();
    if self.config.active_profile.as_deref() == Some(id.as_str()) {
      self.set_active_profile(Some(id));
    }
    self.config.save().await
  }

  /// Read the active session's long-term story state (summary, chapters).
  pub async fn story_state(&self) -> Result<StoryState, EngineError> {
    StoryModule::read(&self.session()?.gamestate).await
  }

  /// Read the active session's raw state for the given gameplay module.
  pub async fn module_state(&self, modid: &str) -> Result<serde_json::Value, EngineError> {
    let Some(module) = self.session()?.gamestate.get(&modid.to_string()) else {
      return Err(EngineError::module_not_found(modid));
    };
    Ok(module.get_all().await?)
  }

  /// Replace the active session's raw state for the given gameplay module.
  /// The module is marked dirty so its agent-facing context re-renders on
  /// the next prompt.
  /// Apply a manual gamestate edit from the sidebar: `keypath` is a full
  /// path (first segment = module ID) and `value` the string override; an
  /// empty string removes the field. The edit applies to the live gamestate
  /// — which is the LAST page's state — and is recorded on the last
  /// retained turn (replacing a previous override of the same keypath) so
  /// replay reproduces it. The GUI only permits edits on the last page.
  pub async fn edit_module_state(
    &mut self,
    keypath: Vec<String>,
    value: String,
  ) -> Result<(), EngineError> {
    if keypath.is_empty() {
      return Err(EngineError::input("empty keypath"));
    }
    let view = self.gamestate()?.view(keypath[0].clone());
    view.apply_edit(&keypath[1..], &value).await?;

    self.session_mut()?.update_turn(|mut turn| {
      turn.edits.retain(|(path, _)| path != &keypath);
      turn.edits.push((keypath.clone(), value.clone()));
      Ok(turn)
    }).await
  }

  pub async fn set_module_state(
    &mut self,
    modid: &str,
    state: serde_json::Value,
  ) -> Result<(), EngineError> {
    let serde_json::Value::Object(storage) = state else {
      return Err(EngineError::input("module state must be a JSON object"));
    };
    let session = self.session_mut()?;
    let Some(module) = session.gamestate.get(&modid.to_string()) else {
      return Err(EngineError::module_not_found(modid));
    };
    module.replace(storage).await;
    session.save().await
  }

  /// The effective prompt the DM agent loop sees on its next run, rendered
  /// for display: the system message (persona, story premise, gameplay
  /// module contexts, player profile) followed by the retained conversation
  /// history.
  pub async fn effective_prompt(&mut self) -> Result<String, EngineError> {
    let modids = self.module_ids();
    self.refresh_module_context_cache(&modids).await?;
    // Render exactly what the DM would receive next: when the agent loop
    // hides tool history, the old tool calls & results are stripped here too.
    let hide_tool_history = self.agent_loop.as_ref()
      .map(|loop_| loop_.hide_tool_history)
      .unwrap_or(false);
    Ok(render_conversation(&self.conversation(&modids)?, hide_tool_history))
  }

  /// Get the active profile. Resolves the [NovelCraftConfig::active_profile]
  /// field from the [NovelCraftConfig::profiles] vector.
  pub fn profile(&self) -> Option<&ProfileV1> {
    self.config.active_profile
      .as_ref()
      .and_then(|id| self.config.profiles.iter().find(|profile| profile.id == *id))
  }

  /// Push a new user prompt to the end of the current session.
  pub async fn prompt(
    &mut self,
    content: String,
    sender: Sender<AgentMessageChunk>,
  ) -> Result<(), EngineError> {
    let modids = self.module_ids();

    self.refresh_module_context_cache(&modids).await?;

    self.session_mut()?.push_turn(TurnV2 {
      prompt: Some(content),
      ..Default::default()
    }).await?;

    let mut conv = self.conversation(&modids)?;
    let prev_msg_len = conv.messages.len();
    let gamestate = self.gamestate()?;

    self.agent_loop
      .as_mut()
      .unwrap()
      .run(
        ContextProvider::factory(|toolset, _| Ok(gamestate.view(toolset.name().to_string()))),
        &mut conv,
        sender,
      ).await?;

    self.session_mut()?.update_turn(|mut turn| {
      turn.assimilate(&conv.messages[prev_msg_len..])?;
      Ok(turn)
    }).await?;

    self.compress_memory_if_due().await;
    Ok(())
  }

  /// Rewrite the last turn under consideration of the given instructions.
  pub async fn rewrite(
    &mut self,
    instruct: String,
    sender: Sender<AgentMessageChunk>,
  ) -> Result<(), EngineError> {
    let modids = self.module_ids();

    self.refresh_module_context_cache(&modids).await?;

    self.session_mut()?.update_turn(move |mut turn| {
      if let Some(system) = turn.system {
        turn.system = Some(format!("{}\n{}", system, instruct));
      } else {
        turn.system = Some(instruct);
      }
      turn.responses.clear();
      Ok(turn)
    }).await?;

    let mut conv = self.conversation(&modids)?;
    let prev_msg_len = conv.messages.len();
    let gamestate = self.gamestate()?;

    self.agent_loop
      .as_mut()
      .unwrap()
      .run(
        ContextProvider::factory(|toolset, _| Ok(gamestate.view(toolset.name().to_string()))),
        &mut conv,
        sender,
      ).await?;

    self.session_mut()?.update_turn(|mut turn| {
      turn.assimilate(&conv.messages[prev_msg_len..])?;
      Ok(turn)
    }).await?;

    self.compress_memory_if_due().await;
    Ok(())
  }

  /// Fork this session from the given retained turn index, deleting all
  /// later turns.
  pub async fn fork(&mut self, turn_index: usize) -> Result<(), EngineError> {
    let profiles = self.config.profiles.clone();
    self.session_mut()?.fork(turn_index, &profiles).await?;
    // NOTE: no need to rebuild agent_loop because tools didn't change
    Ok(())
  }

  /// Drop the entire retained history, restoring the gamestate to the
  /// session snapshot (a fresh start of the story).
  pub async fn reset_history(&mut self) -> Result<(), EngineError> {
    let profiles = self.config.profiles.clone();
    self.session_mut()?.reset_turns(&profiles).await
  }

  /// Overwrite the user prompt of the last retained turn (used by the
  /// GUI's inline prompt editing).
  pub async fn set_last_prompt(&mut self, content: String) -> Result<(), EngineError> {
    self.session_mut()?.update_turn(move |mut turn| {
      turn.prompt = Some(content);
      Ok(turn)
    }).await
  }

  /// Regenerate the active session's last agent response: fork from the
  /// second-to-last turn (dropping the entire history when regenerating the
  /// very first turn) and re-run the DM agent loop on the same user prompt.
  pub async fn regenerate(&mut self, sender: Sender<AgentMessageChunk>) -> Result<(), EngineError> {
    let content = {
      let session = self.session()?;
      let count = session.turn_count();
      let turn = session
        .try_turn(count.checked_sub(1).ok_or_else(|| EngineError::state("session has no turns"))?)
        .ok_or_else(|| EngineError::state("last turn not found"))?;
      turn
        .prompt
        .clone()
        .ok_or_else(|| EngineError::state("last turn has no user prompt to regenerate"))?
    };

    let profiles = self.config.profiles.clone();
    let session = self.session_mut()?;
    if session.turn_count() > 1 {
      session.fork(session.turn_count() - 2, &profiles).await?;
    } else {
      session.reset_turns(&profiles).await?;
    }

    self.prompt(content, sender).await
  }

  /// Compress the retained history into the rolling story summary once it
  /// exceeds `2 * summary_threshold` turns. Failures are logged & retried on
  /// a later trigger — never surfaced to the player.
  async fn compress_memory_if_due(&mut self) {
    let threshold = self.config.summary_threshold;
    let Some(session) = self.session.as_mut() else { return };
    if !session.should_compress(threshold) {
      return;
    }

    let model = self.config.models[ModelPurpose::Summarizer].clone();
    match session.compress_memory(threshold, &model).await {
      Ok(true) => info!("Compressed {threshold} turns into the rolling story summary"),
      Ok(false) => {}
      Err(err) => warn!("Memory compression failed (will retry on a later turn): {err}"),
    }
  }

  async fn refresh_module_context_cache(&mut self, modids: &[String]) -> Result<(), EngineError> {
    let gamestate = self.gamestate()?;
    for modid in modids {
      let dirty = gamestate.mark_clean(modid)? ||
        !self.module_context_cache.contains_key(modid);
      if let Some(module) = self.session()?.module(modid) && dirty {
        let mut f2 = PromptFormatter::new();
        module.context(gamestate.clone(), &mut f2).await?;
        self.module_context_cache.insert(modid.clone(), f2.finish());
      }
    }
    Ok(())
  }

  fn system_prompt_msg(&self, modids: &[String]) -> Result<ConversationMessage, EngineError> {
    let mut f = PromptFormatter::new();

    f.writeline(&self.config.system_prompt)?;
    f.newline()?;

    let exposition = self.session().map(|s| s.exposition.clone()).unwrap_or_default();
    if !exposition.trim().is_empty() {
      f.write("# Story premise")?;
      f.indented(|f| f.write(&exposition))?;
      f.newline()?;
    }

    if !modids.is_empty() {
      f.write("# Gameplay module contexts")?;
      for modid in modids {
        if let Some(module_context) = self.module_context_cache.get(modid)
          && !module_context.trim().is_empty()
        {
          f.indented(|f| f.write_reindent(module_context))?;
        }
      }
      f.newline()?;
    }

    if let Some(profile) = self.profile() {
      f.write("# Player character profile\n\n")?;
      profile.promptify(&mut f)?;
      f.newline()?;
    }

    Ok(ConversationMessage::System { content: f.finish() })
  }

  #[inline(always)]
  fn module_ids(&self) -> Vec<String> {
    let Some(session) = &self.session else { return vec![] };
    session.modules().cloned().collect()
  }
}

/// Render a conversation into a readable, label-prefixed transcript for
/// debugging (as shown in the sidebar's effective prompt section).
/// Render a conversation the way the DM would see it. When
/// `hide_tool_history` is set (mirroring `AgentLoop::hide_tool_history`),
/// tool responses are omitted and tool-call-only assistant messages are
/// skipped, so the display matches the stripped request payload.
fn render_conversation(conv: &Conversation, hide_tool_history: bool) -> String {
  let mut out = String::new();
  for msg in &conv.messages {
    match msg {
      ConversationMessage::System { content } => {
        out.push_str("[system]\n");
        out.push_str(content);
      }
      ConversationMessage::User { content } => {
        out.push_str("\n\n[user]\n");
        out.push_str(content);
      }
      ConversationMessage::Assistant { content, tool_calls } => {
        if hide_tool_history && content.trim().is_empty() {
          // Tool-call carrier message; with the calls hidden it carries
          // nothing the model would pay attention to.
          continue;
        }
        out.push_str("\n\n[assistant]\n");
        out.push_str(content);
        if !hide_tool_history {
          for tc in tool_calls {
            out.push_str(&format!("\n\n[tool call: {}]\n{}", tc.name, tc.arguments));
          }
        }
      }
      ConversationMessage::Tool { id, content } => {
        if hide_tool_history {
          continue;
        }
        out.push_str(&format!("\n\n[tool response {id}]\n{content}"));
      }
    }
  }
  out.push('\n');
  out
}

/// Insert [`TOOL_REMINDER`] as a system message immediately before the
/// latest user prompt (the last `User` message), so the DM is reminded to
/// document the turn right as it receives the player's input. A conversation
/// without any user message is left untouched.
fn inject_tool_reminder(conv: &mut Conversation) {
  let Some(idx) = conv
    .messages
    .iter()
    .rposition(|msg| matches!(msg, ConversationMessage::User { .. }))
  else {
    return;
  };
  conv.messages.insert(idx, ConversationMessage::system(TOOL_REMINDER.to_string()));
}

/// Build a new agent loop. Only needed when `session` changed. When only
/// `config` changed, prefer calling [NovelCraftConfig::contribute] instead.
fn build_agent_loop(
  agent_loop: Option<AgentLoop<GameStateView>>,
  config: &NovelCraftConfig,
  session: Option<&SessionV2>,
) -> Option<AgentLoop<GameStateView>> {
  let mut agent_loop = agent_loop.unwrap_or_default();
  config.contribute(Some(&mut agent_loop));
  // Old tool calls & results are redundant tokens: module contexts and the
  // rolling story summary re-inject the relevant state on every prompt, and
  // the DM's narrative text reflects their outcomes. The DM plans with
  // reasoning (medium effort, OpenRouter-unified `reasoning` parameter) so
  // it documents the turn via its tools before narrating; models without
  // reasoning support ignore the parameter.
  Some(agent_loop
    .with_hide_tool_history(true)
    .with_reasoning(Some(OpenAiReasoningConfig {
      effort: Some("medium".to_string()),
      ..Default::default()
    }))
    .with_toolsets(
      session?.modules
        .values()
        .map(|m| m.clone().toolset())
    ))
}

#[cfg(test)]
mod tests {
  use super::*;

  fn conv(msgs: Vec<ConversationMessage>) -> Conversation {
    Conversation { messages: msgs }
  }

  fn user(content: &str) -> ConversationMessage {
    ConversationMessage::user(content.to_string())
  }

  fn assistant(content: &str) -> ConversationMessage {
    ConversationMessage::assistant(content.to_string(), Vec::new())
  }

  #[test]
  fn reminder_precedes_latest_user_prompt() {
    let mut c = conv(vec![
      ConversationMessage::system("persona".to_string()),
      user("first"),
      assistant("reply"),
      user("second"),
    ]);
    inject_tool_reminder(&mut c);

    assert_eq!(c.messages.len(), 5);
    assert!(matches!(&c.messages[3], ConversationMessage::System { content } if content.contains("timeline")));
    assert!(matches!(&c.messages[4], ConversationMessage::User { content } if content == "second"));
  }

  #[test]
  fn reminder_only_before_last_user_prompt() {
    let mut c = conv(vec![
      ConversationMessage::system("persona".to_string()),
      user("first"),
      assistant("reply"),
      user("second"),
    ]);
    inject_tool_reminder(&mut c);

    // The first user prompt stays directly followed by the assistant reply.
    assert!(matches!(&c.messages[1], ConversationMessage::User { content } if content == "first"));
    assert!(matches!(&c.messages[2], ConversationMessage::Assistant { .. }));
  }

  #[test]
  fn reminder_skips_conversation_without_user_prompts() {
    let mut c = conv(vec![ConversationMessage::system("persona".to_string())]);
    inject_tool_reminder(&mut c);

    assert_eq!(c.messages.len(), 1);
  }
}
