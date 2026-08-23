use std::collections::HashMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use kiruklaw_agent_loop::Conversation;
use serde::{Deserialize, Deserializer, Serialize};
use uuid::Uuid;

use crate::error::AppError;
use crate::game::module::GameplayModule;
use crate::game::pages::PageBatch;
use crate::game::state::GameState;
use crate::util::{deserialize, deserialize_timestamp, serialize_timestamp};

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
  /// Working memory conversation - not exposed for management purposes
  #[serde(skip)]
  pub(crate) conversation: Conversation,
  #[serde(skip)]
  pub(crate) gamestate: GameState,
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
      conversation: Conversation::default(),
      gamestate: GameState::default(),
    }
  }
}

impl SessionV1 {
  pub const VERSION: u8 = 1u8;

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
      PageBatch::load(id.clone(), batch_count - 2),
      PageBatch::load(id.clone(), batch_count - 1),
    );

    let b0 = b0?;
    let b1 = b1?;

    let conv = if batch_count >= 1 {
      let mut conv = b0.to_conversation();
      conv.extend(b1.to_conversation());
      conv
    } else {
      b1.to_conversation()
    };

    res.conversation = conv;
    res.gamestate = GameState::new(b1.snapshot.unwrap_or_default())?;
    res.gamestate.replay(&res.modules, &b1.pages).await?;

    Ok(res)
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

  /// Counts the number of batches in this session. Assumes the local filesystem is not corrupted.
  pub async fn count_batches(session_id: &str) -> Result<usize, AppError> {
    let last_batch_idx = Self::batches(session_id)
      .await?
      .iter()
      .map(|b| PageBatch::parse_batch_idx(b.to_string_lossy()).unwrap_or_default())
      .max()
      .unwrap_or_default();
    Ok(last_batch_idx)
  }

  /// Count the pages in this session. Assumes each batch until the last is at max capacity.
  pub async fn count_pages(session_id: String) -> Result<usize, AppError> {
    let last_batch_idx = Self::count_batches(&session_id).await?;
    let last_batch = PageBatch::load(session_id, last_batch_idx).await?;
    Ok(last_batch_idx * PageBatch::MAX_PAGES_PER_BATCH + last_batch.pages.len())
  }

  fn deserialize_version<'de, D: Deserializer<'de>>(deserializer: D) -> Result<u8, D::Error> {
    let version = u8::deserialize(deserializer)?;
    if version != Self::VERSION {
      return Err(serde::de::Error::custom(format!("Unexpected version {}, expected {}", version, Self::VERSION)));
    }
    Ok(version)
  }
}
