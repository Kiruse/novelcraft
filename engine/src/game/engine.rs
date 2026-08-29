use std::collections::HashMap;
use std::ffi::OsString;
use std::sync::Arc;

use kiruklaw_agent_loop::{AgentLoop, AgentMessageChunk, ContextProvider, Conversation, ConversationMessage};
use moka::future::Cache;
use tokio::sync::mpsc::Sender;

use crate::config::NovelCraftConfig;
use crate::error::AppError;
use crate::game::pages::{PageBatchV1, PageV1};
use crate::game::profile::ProfileV1;
use crate::game::session::SessionV1;
use crate::game::state::{GameState, GameStateView};
use crate::util::prompting::{PromptFormatter, Promptify};

pub struct GameEngine {
  /// NovelCraft game engine config
  config: NovelCraftConfig,
  /// Active user session
  session: Option<SessionV1>,
  /// Cache of module context prompt component strings.
  module_context_cache: HashMap<String, String>,
  /// Cached AgentLoop config
  agent_loop: Option<AgentLoop<GameStateView>>,
  /// LRU cache of page batches, keyed by `(session_id, batch_num)`.
  page_cache: Cache<(String, usize), Arc<PageBatchV1>>,
}

impl GameEngine {
  pub fn new() -> Self {
    Self {
      config: NovelCraftConfig::default(),
      session: None,
      module_context_cache: HashMap::new(),
      agent_loop: None,
      page_cache: Cache::builder()
        .max_capacity(8)
        .build(),
    }
  }

  #[inline]
  pub fn with_config(self, config: NovelCraftConfig) -> Self {
    let agent_loop = Self::build_agent_loop(&config, self.session.as_ref());
    Self {
      config,
      agent_loop,
      ..self
    }
  }
  #[inline]
  pub fn with_session(self, session: SessionV1) -> Self {
    let agent_loop = Self::build_agent_loop(&self.config, Some(&session));
    Self {
      session: Some(session),
      agent_loop,
      ..self
    }
  }
  /// Removes the session from the engine instance
  #[inline]
  pub fn without_session(self) -> Self {
    Self {
      session: None,
      agent_loop: None,
      ..self
    }
  }

  #[inline(always)]
  fn session(&self) -> Result<&SessionV1, AppError> {
    self.session.as_ref().ok_or(AppError::state("no active session"))
  }
  #[inline(always)]
  fn session_mut(&mut self) -> Result<&mut SessionV1, AppError> {
    self.session.as_mut().ok_or(AppError::state("no active session"))
  }
  #[inline(always)]
  fn session_id(&self) -> Result<&String, AppError> {
    Ok(&self.session()?.id)
  }

  #[inline(always)]
  fn gamestate(&self) -> Result<GameState, AppError> {
    self.session().map(|s| s.gamestate.clone())
  }

  pub async fn list_sessions() -> Result<Vec<OsString>, AppError> {
    let path = SessionV1::root()?;
    let mut dir_iter = tokio::fs::read_dir(&path).await?;
    let mut result = Vec::new();
    while let Some(entry) = dir_iter.next_entry().await? {
      result.push(entry.file_name());
    }
    Ok(result)
  }

  fn conversation(&self, modids: &[String]) -> Result<Conversation, AppError> {
    let mut conv = Conversation::default();
    conv.push(self.system_prompt_msg(modids)?);
    conv.extend(self.session()?.conversation());
    Ok(conv)
  }

  pub async fn page(&self, page_index: usize) -> Option<PageV1> {
    let session = self.session.as_ref()?;
    if let Some(page) = session.try_page(page_index) {
      return Some(page.clone());
    }
    let batch_num = PageBatchV1::batch_of(page_index);
    let local_index = PageBatchV1::page_offset(page_index);
    let key = (session.id.clone(), batch_num);
    let sid = session.id.clone();
    let batch = self
      .page_cache
      .optionally_get_with(key, async move {
        PageBatchV1::load(sid, batch_num).await.ok().map(Arc::new)
      })
      .await;
    batch.and_then(|b| b.pages.get(local_index).cloned())
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
  ) -> Result<(), AppError> {
    let modids = self.module_ids();

    self.refresh_module_context_cache(&modids).await?;

    self.session_mut()?.push_page(PageV1 {
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

    self.session_mut()?.update_page(|mut page| {
      page.assimilate(&conv.messages[prev_msg_len..])?;
      Ok(page)
    }).await?;

    Ok(())
  }

  /// Rewrite the last page under consideration of the given instructions.
  pub async fn rewrite(
    &mut self,
    instruct: String,
    sender: Sender<AgentMessageChunk>,
  ) -> Result<(), AppError> {
    let modids = self.module_ids();

    self.refresh_module_context_cache(&modids).await?;

    self.session_mut()?.update_page(move |mut page| {
      if let Some(system) = page.system {
        page.system = Some(format!("{}\n{}", system, instruct));
      } else {
        page.system = Some(instruct);
      }
      page.responses.clear();
      Ok(page)
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

    self.session_mut()?.update_page(|mut page| {
      page.assimilate(&conv.messages[prev_msg_len..])?;
      Ok(page)
    }).await?;

    Ok(())
  }

  /// Fork this session from the given page index, deleting all subsequent pages.
  pub async fn fork(&mut self, page_index: usize) -> Result<(), AppError> {
    let sid = self.session_id()?.clone();
    let batch_index = PageBatchV1::batch_of(page_index);
    Self::truncate_batches(&sid, batch_index).await?;
    let session = SessionV1::load(&sid).await?;
    self.session = Some(session);
    self.agent_loop = Self::build_agent_loop(&self.config, self.session.as_ref());
    Ok(())
  }

  /// Delete all batches after the given `batch_index`, with this `batch_index` becoming
  /// the last retained batch.
  async fn truncate_batches(sid: &String, batch_index: usize) -> Result<(), AppError> {
    let batches = SessionV1::batches(sid).await?;
    let batches = batches
      .iter()
      .map(|b| PageBatchV1::parse_batch_idx(b.to_string_lossy()))
      .filter(Option::is_some)
      .map(|idx| idx.unwrap())
      .filter(|idx| *idx > batch_index);
    let dir = SessionV1::dir(sid)?;
    for batch in batches {
      let path = PageBatchV1::join_path(&dir, batch);
      tokio::fs::remove_file(&path).await?;
    }
    Ok(())
  }

  async fn refresh_module_context_cache(&mut self, modids: &[String]) -> Result<(), AppError> {
    let gamestate = self.gamestate()?;
    for modid in modids {
      let dirty = gamestate.mark_clean(&modid)? ||
        self.module_context_cache.get(modid).is_none();
      if let Some(module) = self.session()?.module(&modid) && dirty {
        let mut f2 = PromptFormatter::new();
        module.context(gamestate.clone(), &mut f2).await?;
        self.module_context_cache.insert(modid.clone(), f2.finish());
      }
    }
    Ok(())
  }

  fn system_prompt_msg(&self, modids: &[String]) -> Result<ConversationMessage, AppError> {
    let mut f = PromptFormatter::new();

    f.writeline(&self.config.system_prompt)?;
    f.newline()?;

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

  /// Build the agent loop from the given config & session.
  ///
  /// Note that the session's gameplay modules will be cloned. They cannot be
  /// borrowed as the underlying [AgentLoop] system uses toolsets as "local
  /// storage," and each loop, including subagents, has its own local storage.
  /// Thus, for optimization, consider using `Arc<...>` in the module config
  /// where values are expected to potentially become very large (e.g. NPCs),
  /// which is generally also fine as modules are deserialized only once, and
  /// henceforth readonly anyways.
  fn build_agent_loop(config: &NovelCraftConfig, session: Option<&SessionV1>) -> Option<AgentLoop<GameStateView>> {
    let Some(session) = session else { return None };
    let model_config = config.models.dungeon_master.clone();
    Some(AgentLoop::<GameStateView>::new(model_config)
      .with_max_steps(20)
      .with_toolsets(
        session.modules
          .values()
          .cloned()
          .map(|module| module.toolset())
      ))
  }

  #[inline(always)]
  fn module_ids(&self) -> Vec<String> {
    let Some(session) = &self.session else { return vec![] };
    session.modules().cloned().collect()
  }
}
