use std::collections::HashMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::{DateTime, Utc};
use kiruklaw_agent_loop::Conversation;
use serde::{Deserialize, Deserializer, Serialize};
use uuid::Uuid;

use crate::error::AppError;
use crate::game::module::GameplayModule;
use crate::game::pages::{PageBatchV1, PageV1};
use crate::game::state::GameState;
use crate::util::{deserialize, deserialize_timestamp, serialize, serialize_timestamp};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionV1 {
  #[serde(deserialize_with = "SessionV1::deserialize_version")]
  pub version: u8,
  pub id: String,
  pub title: String,
  pub description: Option<String>,
  #[serde(serialize_with = "serialize_timestamp", deserialize_with = "deserialize_timestamp")]
  #[serde(rename = "created_at")]
  pub created_at: DateTime<Utc>,
  #[serde(serialize_with = "serialize_timestamp", deserialize_with = "deserialize_timestamp")]
  #[serde(rename = "updated_at")]
  pub updated_at: DateTime<Utc>,
  /// Configured gameplay modules that this session uses.
  pub modules: HashMap<String, GameplayModule>,
  #[serde(skip_deserializing)]
  pub page_count: usize,
  /// Total number of page batches in this session
  #[serde(skip)]
  pub batch_count: usize,
  /// Last 2 batches of pages in the session. In a fresh session,
  /// the second batch may be default.
  #[serde(skip)]
  tail_batches: (PageBatchV1, PageBatchV1),
  #[serde(skip)]
  pub(crate) gamestate: Arc<GameState>,
}

impl Default for SessionV1 {
  fn default() -> Self {
    Self {
      version: Self::VERSION,
      id: Uuid::new_v4().to_string(),
      title: String::new(),
      description: None,
      created_at: Utc::now(),
      updated_at: Utc::now(),
      modules: Default::default(),
      page_count: 0,
      batch_count: 0,
      tail_batches: (PageBatchV1::default(), PageBatchV1::default()),
      gamestate: Arc::new(GameState::default()),
    }
  }
}

impl SessionV1 {
  pub const VERSION: u8 = 1u8;

  pub fn new(id: String) -> Self {
    Self {
      version: Self::VERSION,
      id,
      ..Default::default()
    }
  }

  pub fn root() -> Result<PathBuf, AppError> {
    crate::paths::sessions_dir()
  }

  pub fn dir(id: &str) -> Result<PathBuf, AppError> {
    Ok(Self::root()?.join(id))
  }

  pub fn meta_path(id: &str) -> Result<PathBuf, AppError> {
    Ok(Self::join_meta_path(&Self::dir(id)?))
  }

  fn join_meta_path(dir: &Path) -> PathBuf {
    dir.join("meta.json")
  }

  /// Restore the given session by `id` from the local filesystem.
  pub async fn load(id: impl Into<String>) -> Result<SessionV1, AppError> {
    let id = id.into();
    let path = Self::meta_path(&id)?;
    let mut res: SessionV1 = deserialize(&path).await?;

    let batch_count = Self::count_batches(&id).await?;
    res.page_count = Self::count_pages(id.clone()).await?;
    res.batch_count = batch_count;

    let (b0, b1) = tokio::join!(
      PageBatchV1::load(id.clone(), batch_count - 2),
      PageBatchV1::load(id.clone(), batch_count - 1),
    );

    let batch = match batch_count {
      0 => {
        res.tail_batches = (PageBatchV1::default(), PageBatchV1::default());
        &res.tail_batches.0
      }
      1 => {
        res.tail_batches = (b1?, PageBatchV1::default());
        &res.tail_batches.0
      }
      _ => {
        res.tail_batches = (b0?, b1?);
        &res.tail_batches.1
      }
    };

    res.gamestate = Arc::new(GameState::new(batch.snapshot.clone())?);
    res.gamestate.replay(&res.modules, &batch.pages).await?;

    Ok(res)
  }

  /// Save this session to disk, including its last 2 associated page batches
  /// (which are considered working memory).
  pub async fn save(&self) -> Result<(), AppError> {
    let (r_self, r_b1, r_b2) = tokio::join!(
      self.save_metadata(),
      self.tail_batches.0.save(),
      self.tail_batches.1.save(),
    );
    r_self?;
    r_b1?;
    r_b2?;
    Ok(())
  }

  /// Save only this session's metadata, not its [SessionV1::tail_batches].
  async fn save_metadata(&self) -> Result<(), AppError> {
    let path = Self::meta_path(&self.id)?;
    serialize(&path, self).await?;
    Ok(())
  }

  /// Enumerate all saved sessions on the local filesystem
  pub async fn list() -> Result<Vec<SessionV1>, AppError> {
    let dir = Self::root()?;
    let mut entry_iter = tokio::fs::read_dir(&dir).await?;
    let mut res: Vec<SessionV1> = Vec::new();
    while let Some(entry) = entry_iter.next_entry().await? {
      if !entry.file_type().await.map(|ty| ty.is_dir()).unwrap_or(false) {
        continue;
      }

      let sid = entry.file_name().to_string_lossy().to_string();
      res.push(SessionV1::load(&sid).await?);
    }

    Ok(res)
  }

  /// Enumerate batches of this session on the local filesystem
  pub async fn batches(session_id: &str) -> Result<Vec<OsString>, AppError> {
    let dir = Self::dir(session_id)?;
    let mut dir_iter = tokio::fs::read_dir(&dir).await?;
    let mut result = Vec::new();
    while let Some(entry) = dir_iter.next_entry().await? {
      let filename = entry.file_name();
      let filename = filename.to_string_lossy();
      if filename.starts_with("pages.") && filename.ends_with(".json") {
        result.push(entry.file_name());
      }
    }
    Ok(result)
  }

  #[inline]
  fn active_batch(&self) -> &PageBatchV1 {
    if !self.tail_batches.0.is_full() {
      &self.tail_batches.0
    } else {
      &self.tail_batches.1
    }
  }

  #[inline]
  fn active_batch_mut(&mut self) -> &mut PageBatchV1 {
    if !self.tail_batches.0.is_full() {
      &mut self.tail_batches.0
    } else {
      &mut self.tail_batches.1
    }
  }

  /// Counts the number of batches in this session. Assumes the local filesystem is not corrupted.
  pub async fn count_batches(session_id: &str) -> Result<usize, AppError> {
    let last_batch_idx = Self::batches(session_id)
      .await?
      .iter()
      .map(|b| PageBatchV1::parse_batch_idx(b.to_string_lossy()).unwrap_or_default())
      .max()
      .unwrap_or_default();
    Ok(last_batch_idx)
  }

  /// Count the pages in this session. Assumes each batch until the last is at max capacity.
  pub async fn count_pages(session_id: String) -> Result<usize, AppError> {
    let last_batch_idx = Self::count_batches(&session_id).await?;
    let last_batch = PageBatchV1::load(session_id, last_batch_idx).await?;
    Ok(last_batch_idx * PageBatchV1::MAX_PAGES_PER_BATCH + last_batch.pages.len())
  }

  /// Get an iterator over the module IDs of this session
  pub fn modules(&self) -> impl Iterator<Item = &String> {
    self.modules.keys()
  }

  /// Get a module by its ID, if any
  pub fn module(&self, module: &String) -> Option<&GameplayModule> {
    self.modules.get(module)
  }

  /// Push a new page to the end of the session.
  pub async fn push_page(&mut self, page: PageV1) -> Result<(), AppError> {
    // active batch will never be full if we only have 1 batch
    if self.active_batch().is_full() {
      self.tail_batches.0.save().await?;
      std::mem::swap(&mut self.tail_batches.0, &mut self.tail_batches.1);
      self.tail_batches.1 = PageBatchV1::new(
        self.id.clone(),
        self.batch_count,
      );
      self.batch_count += 1;
    }

    // when inserting first page, also save gamestate snapshot
    if self.active_batch().is_empty() {
      self.active_batch_mut().snapshot = self.gamestate.snapshot().await;
    }

    self.active_batch_mut().pages.push(page);
    self.page_count += 1;

    self.save().await
  }

  /// Update the last page of the session.
  pub async fn update_page(&mut self, cb: impl FnOnce(PageV1) -> PageV1) -> Result<(), AppError> {
    let batch = self.active_batch_mut();
    let page = batch.pages.pop().ok_or(AppError::state("fresh session"))?;
    batch.pages.push(cb(page));
    self.save().await
  }

  /// Replace the last page of the session with the given page.
  pub async fn replace_page(&mut self, page: PageV1) -> Result<(), AppError> {
    let batch = self.active_batch_mut();
    batch.pages.pop();
    batch.pages.push(page);
    self.save().await
  }

  /// Drop the last page of the session.
  pub async fn pop_page(&mut self) -> Result<Option<PageV1>, AppError> {
    self.updated_at = Utc::now();
    self.save_metadata().await?;

    if let Some(p) = self.tail_batches.1.pages.pop() {
      self.tail_batches.1.save().await?;
      Ok(Some(p))
    } else if let Some(p) = self.tail_batches.0.pages.pop() {
      self.tail_batches.0.save().await?;
      Ok(Some(p))
    } else {
      Ok(None)
    }
  }

  fn deserialize_version<'de, D: Deserializer<'de>>(deserializer: D) -> Result<u8, D::Error> {
    let version = u8::deserialize(deserializer)?;
    if version != Self::VERSION {
      return Err(serde::de::Error::custom(format!("Unexpected version {}, expected {}", version, Self::VERSION)));
    }
    Ok(version)
  }

  pub(crate) fn conversation(&self) -> Conversation {
    let mut conv = self.tail_batches.0.to_conversation();
    conv.extend(self.tail_batches.1.to_conversation());
    conv
  }
}
