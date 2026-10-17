use std::collections::HashMap;
use std::sync::Arc;

use kiruklaw_agent_loop::{AgentLoop, AgentMessageChunk, ContextProvider, Conversation, ConversationMessage};
use log::{info, warn};
use tokio::sync::mpsc::Sender;

use crate::config::{ModelPurpose, NovelCraftConfig};
use crate::error::EngineError;
use crate::game::module::memory::{MemoryModule, MemoryState};
use crate::game::profile::ProfileV1;
use crate::game::session::SessionV2;
use crate::game::state::{GameState, GameStateView};
use crate::game::turns::TurnV2;
use crate::util::prompting::{PromptFormatter, Promptify};

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
    if !path.exists() {
      tokio::fs::create_dir_all(&path).await?;
    }
    let mut dir_iter = tokio::fs::read_dir(&path).await?;
    let mut result = Vec::new();
    while let Some(entry) = dir_iter.next_entry().await? {
      let ty = entry.file_type().await?;
      let name = entry.file_name().to_string_lossy().to_string();
      if ty.is_dir() {
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
    Ok(conv)
  }

  /// Get a retained turn by its window-relative index, if any.
  pub fn turn(&self, index: usize) -> Option<TurnV2> {
    self.session.as_ref()?.try_turn(index).cloned()
  }

  /// Read the active session's long-term memory state.
  pub async fn memory(&self) -> Result<MemoryState, EngineError> {
    MemoryModule::read(&self.session()?.gamestate).await
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
        if let Some(module_context) = self.module_context_cache.get(modid) {
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

/// Build a new agent loop. Only needed when `session` changed. When only
/// `config` changed, prefer calling [NovelCraftConfig::contribute] instead.
fn build_agent_loop(
  agent_loop: Option<AgentLoop<GameStateView>>,
  config: &NovelCraftConfig,
  session: Option<&SessionV2>,
) -> Option<AgentLoop<GameStateView>> {
  let mut agent_loop = agent_loop.unwrap_or_default();
  config.contribute(Some(&mut agent_loop));
  Some(agent_loop
    .with_toolsets(
      session?.modules
        .values()
        .map(|m| m.clone().toolset())
    ))
}
