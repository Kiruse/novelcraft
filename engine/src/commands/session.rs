use chrono::Utc;
use log::warn;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use uuid::Uuid;

use crate::commands::paths as cmd_paths;
use crate::error::AppError;
use crate::util;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionMeta {
  pub version: u32,
  pub id: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub story_id: Option<String>,
  pub title: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub description: Option<String>,
  pub created_at: String,
  pub updated_at: String,
}

impl SessionMeta {
  pub fn validate(&self) -> Result<(), AppError> {
    if self.version != 1 {
      return Err(AppError::validation("Invalid SessionMeta version"));
    }
    if chrono::DateTime::parse_from_rfc3339(self.created_at.as_str()).is_err() {
      return Err(AppError::validation("Invalid created_at timestamp"));
    }
    if chrono::DateTime::parse_from_rfc3339(&self.updated_at.as_str()).is_err() {
      return Err(AppError::validation("Invalid updated_at timestamp"));
    }
    Ok(())
  }

  pub fn touch(&mut self) {
    self.updated_at = Utc::now().to_string();
  }

  pub fn root() -> Result<PathBuf, AppError> {
    cmd_paths::sessions_dir()
  }

  pub fn dir(id: &str) -> Result<PathBuf, AppError> {
    Ok(Self::root()?.join(id))
  }

  pub fn dir_exists(id: &str) -> Result<PathBuf, AppError> {
    let dir = Self::dir(id)?;
    if !dir.exists() {
      Err(AppError::not_found(format!("Session {} not found", id)))
    } else {
      Ok(dir)
    }
  }

  pub fn join_path(path: &Path) -> PathBuf {
    path.join("meta.json")
  }
}

impl Default for SessionMeta {
  fn default() -> Self {
    let ts = Utc::now();
    SessionMeta {
      version: 1,
      id: Uuid::new_v4().to_string(),
      story_id: None,
      title: "".to_string(),
      description: None,
      created_at: ts.to_string(),
      updated_at: ts.to_string(),
    }
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PageEntry {
  pub id: String,
  pub session_id: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub system: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub prompt: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub response: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub tool_calls: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PagesBatch {
  version: u32,
  pages: Vec<PageEntry>,
}

impl PagesBatch {
  pub const SIZE: usize = 100;

  pub fn idx(page_idx: usize) -> usize {
    page_idx / Self::SIZE
  }

  pub fn offset(page_idx: usize) -> usize {
    page_idx % Self::SIZE
  }

  pub fn path(dir: &Path, batch: usize) -> PathBuf {
    dir.join(format!("pages.{:03}.json", batch))
  }
}

impl Default for PagesBatch {
  fn default() -> Self {
    Self {
      version: 1,
      pages: Vec::new(),
    }
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
  pub version: u32,
  pub id: String,
  pub session_id: String,
  pub page_index: u32,
  pub data: serde_json::Value,
}

impl Snapshot {
  pub fn validate(&self) -> Result<(), AppError> {
    if self.version != 1 {
      return Err(AppError::validation("Unexpected version"));
    }
    Ok(())
  }

  pub fn join_head_path(dir: &Path) -> PathBuf {
    dir.join("state.head.json")
  }

  pub fn join_path(dir: &Path, batch: usize) -> PathBuf {
    dir.join(format!("state.{:03}.json", batch))
  }
}

impl Default for Snapshot {
  fn default() -> Self {
    Self {
      version: 1,
      id: Uuid::new_v4().to_string(),
      session_id: "".to_string(),
      page_index: 0,
      data: serde_json::json!({}),
    }
  }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionLoadResult {
  pub meta: SessionMeta,
  pub pages: Vec<PageEntry>,
}

async fn total_page_count(dir: &std::path::Path) -> Result<usize, AppError> {
  if !dir.exists() {
    return Ok(0);
  }

  let mut entries = tokio::fs::read_dir(dir).await?;
  let mut max_batch: Option<usize> = None;

  while let Some(entry) = entries.next_entry().await? {
    let name = entry.file_name().to_string_lossy().to_string();
    if name.starts_with("pages.") && name.ends_with(".json") {
      let num_str = name.trim_start_matches("pages.").trim_end_matches(".json");
      if let Ok(num) = num_str.parse::<usize>() {
        max_batch = Some(max_batch.map_or(num, |m| m.max(num)));
      } else {
        warn!("Invalid batch name {}", name);
      }
    }
  }

  match max_batch {
    None => Ok(0),
    Some(bi) => {
      let batch: PagesBatch = util::deserialize(&PagesBatch::path(dir, bi)).await?;
      Ok(bi * PagesBatch::SIZE + batch.pages.len())
    }
  }
}

async fn read_all_pages(dir: &std::path::Path) -> Result<Vec<PageEntry>, AppError> {
  let mut entries = tokio::fs::read_dir(dir).await?;
  let mut batches: Vec<(u32, PagesBatch)> = Vec::new();

  while let Some(entry) = entries.next_entry().await? {
    let name = entry.file_name().to_string_lossy().to_string();
    if name.starts_with("pages.") && name.ends_with(".json") {
      let num_str = name.trim_start_matches("pages.").trim_end_matches(".json");
      if let Ok(bi) = num_str.parse::<u32>() {
        let batch: PagesBatch = util::deserialize(&entry.path()).await?;
        batches.push((bi, batch));
      } else {
        warn!("Invalid batch name {}", name);
      }
    }
  }

  batches.sort_by_key(|(idx, _)| *idx);
  let mut pages = Vec::new();
  for (_, batch) in batches {
    pages.extend(batch.pages);
  }
  Ok(pages)
}

pub async fn session_list() -> Result<Vec<SessionMeta>, AppError> {
  let dir = SessionMeta::root()?;
  if !dir.exists() {
    return Ok(Vec::new());
  }

  let mut entries = tokio::fs::read_dir(&dir).await?;
  let mut sessions = Vec::new();

  while let Some(entry) = entries.next_entry().await? {
    if entry.file_type().await.map(|t| t.is_dir()).unwrap_or(false) {
      let mp = SessionMeta::join_path(&entry.path());
      if mp.exists() {
        let meta: SessionMeta = util::deserialize(&mp).await?;
        sessions.push(meta);
      }
    }
  }

  sessions.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
  Ok(sessions)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionCreateRequest {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub story_id: Option<String>,
  pub title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionCreateResult {
  pub session_id: String,
  pub snapshot_id: String,
}

pub async fn session_create(
  req: SessionCreateRequest,
) -> Result<SessionCreateResult, AppError> {
  let session_id = Uuid::new_v4().to_string();
  let dir = SessionMeta::dir(&session_id)?;
  util::ensure_dir(&dir).await?;

  let meta = SessionMeta {
    id: session_id.clone(),
    story_id: req.story_id,
    title: req.title,
    ..Default::default()
  };
  meta.validate()?;
  util::serialize(&SessionMeta::join_path(&dir), &meta).await?;

  let head = Snapshot {
    id: Uuid::new_v4().to_string(),
    session_id: meta.id.clone(),
    ..Default::default()
  };
  head.validate()?;
  util::serialize(&Snapshot::join_head_path(&dir), &head).await?;

  Ok(SessionCreateResult {
    session_id: meta.id,
    snapshot_id: head.id,
  })
}

pub async fn session_delete(id: String) -> Result<(), AppError> {
  let dir = SessionMeta::dir(&id)?;
  if dir.exists() {
    tokio::fs::remove_dir_all(&dir).await?;
  }
  Ok(())
}

pub async fn session_load(id: String) -> Result<SessionLoadResult, AppError> {
  let dir = SessionMeta::dir_exists(&id)?;
  let meta: SessionMeta = util::deserialize(&SessionMeta::join_path(&dir)).await?;
  let pages = read_all_pages(&dir).await?;
  Ok(SessionLoadResult { meta, pages })
}

pub async fn session_save_meta(
  mut meta: SessionMeta,
) -> Result<(), AppError> {
  meta.touch();
  meta.validate()?;

  let dir = SessionMeta::dir(&meta.id)?;
  util::ensure_dir(&dir).await?;
  util::serialize(&SessionMeta::join_path(&dir), &meta).await
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionUpsertPageRequest {
  pub session_id: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub page_index: Option<u32>,
  pub page: PageEntry,
}

pub async fn session_upsert_page(
  req: SessionUpsertPageRequest,
) -> Result<(), AppError> {
  let dir = SessionMeta::dir_exists(&req.session_id)?;

  match req.page_index {
    None => {
      let count = total_page_count(&dir).await?;
      let bi = PagesBatch::idx(count);
      let path = PagesBatch::path(&dir, bi);

      let mut batch: PagesBatch = util::deserialize(&path).await.unwrap_or_default();
      batch.pages.push(req.page);
      util::serialize(&path, &batch).await?;
    }
    Some(idx) => {
      let bi = PagesBatch::idx(idx as usize);
      let li = PagesBatch::offset(idx as usize);
      let path = PagesBatch::path(&dir, bi);

      let mut batch: PagesBatch = util::deserialize(&path).await
        .map_err(|e| AppError::not_found(format!("Page batch {} not found: {}", bi, e)))?;

      if li >= batch.pages.len() {
        return Err(AppError::not_found(format!(
          "Page index {} out of bounds in batch {}",
          li, bi
        )));
      }

      batch.pages[li] = req.page;
      util::serialize(&path, &batch).await?;
    }
  }

  let meta_path = SessionMeta::join_path(&dir);
  let mut meta: SessionMeta = util::deserialize(&meta_path).await.unwrap_or_default();
  meta.touch();
  util::serialize(&meta_path, &meta).await?;

  Ok(())
}

pub async fn session_truncate_pages(
  session_id: String,
  from_index: u32,
) -> Result<(), AppError> {
  let dir = SessionMeta::dir_exists(&session_id)?;

  let from_bi = PagesBatch::idx(from_index as usize);
  let li = PagesBatch::offset(from_index as usize);

  let path = PagesBatch::path(&dir, from_bi);
  if path.exists() {
    let mut batch: PagesBatch = util::deserialize(&path).await?;
    batch.pages.truncate(li);
    if batch.pages.is_empty() {
      tokio::fs::remove_file(&path).await?;
    } else {
      util::serialize(&path, &batch).await?;
    }
  }

  let mut bi = from_bi + 1;
  loop {
    let p = PagesBatch::path(&dir, bi);
    if p.exists() {
      tokio::fs::remove_file(&p).await?;
      bi += 1;
    } else {
      break;
    }
  }

  Ok(())
}

pub async fn session_get_head_snapshot(
  session_id: String,
) -> Result<Option<Snapshot>, AppError> {
  let dir = SessionMeta::dir(&session_id)?;
  let path = Snapshot::join_head_path(&dir);
  if path.exists() {
    Ok(Some(util::deserialize(&path).await?))
  } else {
    Ok(None)
  }
}

pub async fn session_save_head_snapshot(
  session_id: String,
  snapshot: Snapshot,
) -> Result<(), AppError> {
  let dir = SessionMeta::dir_exists(&session_id)?;
  util::serialize(&Snapshot::join_head_path(&dir), &snapshot).await
}

pub async fn session_delete_head_snapshot(
  session_id: String,
) -> Result<(), AppError> {
  let dir = SessionMeta::dir(&session_id)?;
  let path = Snapshot::join_head_path(&dir);
  if path.exists() {
    tokio::fs::remove_file(&path).await?;
  }
  Ok(())
}

pub async fn session_find_snapshot_before(
  session_id: String,
  page_index: u32,
) -> Result<Option<Snapshot>, AppError> {
  let dir = SessionMeta::dir(&session_id)?;
  let max_batch = PagesBatch::idx(page_index as usize);

  for bi in (0..=max_batch).rev() {
    let path = Snapshot::join_path(&dir, bi);
    if path.exists() {
      let snap: Snapshot = util::deserialize(&path).await?;
      if snap.page_index <= page_index {
        return Ok(Some(snap));
      }
    }
  }

  Ok(None)
}

pub async fn session_save_checkpoint(
  session_id: String,
  snapshot: Snapshot,
) -> Result<(), AppError> {
  let dir = SessionMeta::dir_exists(&session_id)?;
  let bi = PagesBatch::idx(snapshot.page_index as usize);
  util::serialize(&Snapshot::join_path(&dir, bi), &snapshot).await
}

pub async fn session_delete_checkpoints_from(
  session_id: String,
  from_page_index: u32,
) -> Result<(), AppError> {
  let dir = SessionMeta::dir_exists(&session_id)?;
  let first_batch = (from_page_index as usize).div_ceil(PagesBatch::SIZE);

  let mut bi = first_batch;
  loop {
    let path = Snapshot::join_path(&dir, bi);
    if path.exists() {
      tokio::fs::remove_file(&path).await?;
      bi += 1;
    } else {
      break;
    }
  }
  Ok(())
}
