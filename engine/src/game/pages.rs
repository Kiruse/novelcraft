use std::path::{Path, PathBuf};

use kiruklaw_agent_loop::{Conversation, ConversationMessage, ToolCall};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Map;

use crate::error::AppError;
use crate::game::session::SessionV1;
use crate::util::{deserialize, serialize};

pub type Snapshot = Map<String, serde_json::Value>;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct PageBatchRecordV1 {
  #[serde(deserialize_with = "PageBatchRecordV1::deserialize_version")]
  pub version: u8,
  pub pages: Vec<PageV1>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub snapshot: Option<Snapshot>,
}

impl PageBatchRecordV1 {
  pub const VERSION: u8 = 1u8;

  fn to_page_batch(self, session_id: String, offset: usize) -> PageBatch {
    PageBatch {
      session_id,
      offset,
      pages: self.pages,
      snapshot: self.snapshot,
    }
  }

  /// Specialized serializer which simply reads a u8 & validates it to match our expected version
  fn deserialize_version<'de, D: Deserializer<'de>>(deserializer: D) -> Result<u8, D::Error> {
    let version = u8::deserialize(deserializer)?;
    if version != Self::VERSION {
      return Err(serde::de::Error::custom(format!("Invalid version {}, expected {}", version, Self::VERSION)));
    }
    Ok(version)
  }
}

impl From<&PageBatch> for PageBatchRecordV1 {
  fn from(value: &PageBatch) -> Self {
    Self {
      version: Self::VERSION,
      pages: value.pages.clone(),
      snapshot: value.snapshot.clone(),
    }
  }
}

#[derive(Debug, Clone, Default)]
pub struct PageBatch {
  pub session_id: String,
  pub offset: usize,
  pub pages: Vec<PageV1>,
  pub snapshot: Option<Snapshot>,
}

impl PageBatch {
  pub const MAX_PAGES_PER_BATCH: usize = 100;

  pub fn path(session_id: &str, batch: usize) -> Result<PathBuf, AppError> {
    Ok(Self::join_path(&SessionV1::dir(session_id)?, batch))
  }

  pub fn join_path(dir: &Path, batch: usize) -> PathBuf {
    dir.join(format!("pages.{:03}.json", batch))
  }

  pub async fn load(session_id: String, batch: usize) -> Result<PageBatch, AppError> {
    let path = Self::path(&session_id, batch)?;
    let rec: PageBatchRecordV1 = deserialize(&path).await?;
    Ok(rec.to_page_batch(session_id, batch))
  }

  pub async fn save(&self) -> Result<(), AppError> {
    let path = Self::path(&self.session_id, self.offset)?;
    serialize(&path, &PageBatchRecordV1::from(self)).await?;
    Ok(())
  }

  pub fn is_full(&self) -> bool {
    self.pages.len() >= Self::MAX_PAGES_PER_BATCH
  }

  pub fn batch_of(page_index: usize) -> usize {
    page_index / Self::MAX_PAGES_PER_BATCH
  }

  pub fn parse_batch_idx(str: impl AsRef<str>) -> Option<usize> {
    let str = str.as_ref();
    let idx_str = str.trim_start_matches("pages.").trim_end_matches(".json");
    idx_str.parse::<usize>().ok()
  }

  pub fn page_offset(page_index: usize) -> usize {
    page_index % Self::MAX_PAGES_PER_BATCH
  }

  pub fn eq_idx(lhs: &PageBatch, rhs: &PageBatch) -> bool {
    lhs.session_id == rhs.session_id && lhs.offset == rhs.offset
  }

  pub fn to_conversation(&self) -> Conversation {
    let mut conv = Conversation::default();
    conv.messages.reserve(100);
    for page in &self.pages {
      if let Some(system) = &page.system {
        conv.push(ConversationMessage::system(system.clone()));
      }
      if let Some(prompt) = &page.prompt {
        conv.push(ConversationMessage::user(prompt.clone()));
      }
      if let Some(response) = &page.response {
        conv.push(ConversationMessage::assistant(response.clone(), page.tool_calls.clone()));
        conv.messages.reserve(page.tool_responses.len());
        for (id, response) in &page.tool_responses {
          conv.push(ConversationMessage::tool(id.clone(), response.clone()));
        }
      }
    }
    conv
  }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PageV1 {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub system: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub prompt: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub response: Option<String>,
  #[serde(skip_serializing_if = "Vec::is_empty")]
  pub tool_calls: Vec<ToolCall>,
  #[serde(skip_serializing_if = "Vec::is_empty")]
  pub tool_responses: Vec<(String, String)>,
}
