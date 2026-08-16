use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Map;

use crate::error::AppError;
use crate::game::session::SessionV1;
use crate::infer::openai::OpenAiChatMessage;
use crate::infer::internal::ToolCall;
use crate::util::{deserialize, serialize};

pub type Snapshot = Map<String, serde_json::Value>;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct PageBatchRecord {
  pub pages: Vec<PageV1>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub snapshot: Option<Snapshot>,
}

impl From<&PageBatch> for PageBatchRecord {
  fn from(value: &PageBatch) -> Self {
    Self {
      pages: value.pages.clone(),
      snapshot: value.snapshot.clone(),
    }
  }
}

impl PageBatchRecord {
  fn to_page_batch(self, session_id: String, offset: usize) -> PageBatch {
    PageBatch {
      session_id,
      offset,
      pages: self.pages,
      snapshot: self.snapshot,
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
    let rec: PageBatchRecord = deserialize(&path).await?;
    Ok(rec.to_page_batch(session_id, batch))
  }

  pub async fn save(&self) -> Result<(), AppError> {
    let path = Self::path(&self.session_id, self.offset)?;
    serialize(&path, &PageBatchRecord::from(self)).await?;
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
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PageV1 {
  pub system: Option<String>,
  pub prompt: Option<String>,
  pub responses: Vec<ResponseV1>,
}

impl PageV1 {
  pub fn to_openai_messages(pages: &Vec<&PageV1>) -> Vec<OpenAiChatMessage> {
    let mut res: Vec<OpenAiChatMessage> = Vec::new();
    for page in pages {
      if let Some(system) = &page.system {
        res.push(OpenAiChatMessage::system(system.clone()));
      }
      if let Some(prompt) = &page.prompt {
        res.push(OpenAiChatMessage::user(prompt.clone()));
      }
      for response in page.responses.iter() {
        if !response.tool_calls.is_empty() {
          res.push(OpenAiChatMessage::toolcall(
            response.content.clone(),
            response.tool_calls
              .iter()
              .map(|tc| tc.clone().into())
              .collect::<Vec<_>>()
          ));
          if let Some(results) = &response.tool_results {
            res.extend(results
              .iter()
              .map(|r| OpenAiChatMessage::toolresult(r.id.clone(), r.result.clone())))
          }
        }
      }
    }
    res
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseV1 {
  pub content: Option<String>,
  pub tool_calls: Vec<ToolCall>,
  pub tool_results: Option<Vec<ToolResult>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
  pub id: String,
  pub result: Result<String, String>,
}
