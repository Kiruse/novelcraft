use std::ffi::OsString;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{AppHandle, Manager};
use uuid::Uuid;

use crate::error::AppError;
use crate::game::pages::PageBatch;
use crate::util::{deserialize, deserialize_timestamp, serialize, serialize_timestamp};

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct SessionV1 {
  pub id: String,
  pub title: String,
  pub description: Option<String>,
  #[serde(serialize_with = "serialize_timestamp", deserialize_with = "deserialize_timestamp")]
  #[specta(type = String)]
  pub created_at: DateTime<Utc>,
  #[serde(serialize_with = "serialize_timestamp", deserialize_with = "deserialize_timestamp")]
  #[specta(type = String)]
  pub updated_at: DateTime<Utc>,
  // ephemeral
  #[serde(skip_deserializing)]
  pub page_count: usize,
  #[serde(skip)]
  pub batch_count: usize,
}

impl Default for SessionV1 {
  fn default() -> Self {
    Self {
      id: Uuid::new_v4().to_string(),
      title: String::new(),
      description: None,
      created_at: Utc::now(),
      updated_at: Utc::now(),
      page_count: 0,
      batch_count: 0,
    }
  }
}

impl SessionV1 {
  pub fn root(app: &AppHandle) -> Result<PathBuf, AppError> {
    Ok(app
      .path()
      .app_data_dir()?
      .join("sessions"))
  }

  pub fn dir(app: &AppHandle, id: &String) -> Result<PathBuf, AppError> {
    Ok(Self::root(app)?.join(id))
  }

  pub fn meta_path(app: &AppHandle, id: &String) -> Result<PathBuf, AppError> {
    Ok(Self::join_meta_path(&Self::dir(app, id)?))
  }
  fn join_meta_path(dir: &Path) -> PathBuf {
    dir.join("meta.json")
  }

  pub async fn load(app: &AppHandle, id: &String) -> Result<SessionV1, AppError> {
    let path = Self::meta_path(&app, id)?;
    let mut res: SessionV1 = deserialize(&path).await?;
    res.page_count = Self::count_pages(app, id.clone()).await?;
    Ok(res)
  }

  pub async fn save(&self, app: &AppHandle) -> Result<(), AppError> {
    let path = Self::meta_path(app, &self.id)?;
    serialize(&path, self).await?;
    Ok(())
  }

  /// List all sessions saved on disk.
  pub async fn list(app: &AppHandle) -> Result<Vec<SessionV1>, AppError> {
    let dir = Self::root(app)?;
    let mut entry_iter = tokio::fs::read_dir(&dir).await?;
    let mut res: Vec<SessionV1> = Vec::new();
    while let Some(entry) = entry_iter.next_entry().await? {
      if !entry.file_type().await.map(|ty| ty.is_dir()).unwrap_or(false) {
        continue;
      }

      let sid = entry.file_name().to_string_lossy().to_string();
      res.push(SessionV1::load(app, &sid).await?);
    }

    Ok(res)
  }

  /// List the page batch filenames for the given session by ID.
  pub async fn batches(app: &AppHandle, session_id: &String) -> Result<Vec<OsString>, AppError> {
    let dir = Self::dir(app, &session_id)?;
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

  pub async fn count_batches(app: &AppHandle, session_id: &String) -> Result<usize, AppError> {
    let last_batch_idx = Self::batches(app, &session_id)
      .await?
      .iter()
      .map(|b| PageBatch::parse_batch_idx(b.to_string_lossy()).unwrap_or_default())
      .max()
      .unwrap_or_default();
    Ok(last_batch_idx)
  }
  pub async fn count_pages(app: &AppHandle, session_id: String) -> Result<usize, AppError> {
    let last_batch_idx = Self::count_batches(app, &session_id).await?;
    Self::count_pages_with_batch_count(app, session_id, last_batch_idx).await
  }
  async fn count_pages_with_batch_count(app: &AppHandle, session_id: String, last_batch_idx: usize) -> Result<usize, AppError> {
    let last_batch = PageBatch::load(app, session_id, last_batch_idx).await?;
    Ok(last_batch_idx * PageBatch::MAX_PAGES_PER_BATCH + last_batch.pages.len())
  }
}
