use std::collections::HashSet;
use std::path::{Path, PathBuf};

use kiruklaw_agent_loop::{Conversation, ConversationMessage, ToolCall};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Map;

use crate::error::EngineError;
use crate::game::session::SessionV1;
use crate::util::{deserialize, serialize};

pub type Snapshot = Map<String, serde_json::Value>;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PageBatchV1 {
  #[serde(deserialize_with = "PageBatchV1::deserialize_version")]
  pub version: u8,
  /// Session ID this batch belongs to
  #[serde(skip)]
  pub session_id: String,
  /// Number of this batch within its session
  #[serde(skip)]
  pub batch_num: usize,
  /// Pages contained in this batch
  pub pages: Vec<PageV1>,
  /// Snapshot at the start of the page batch, for replay purposes
  pub snapshot: Snapshot,
}

impl PageBatchV1 {
  pub const VERSION: u8 = 1u8;
  pub const MAX_PAGES_PER_BATCH: usize = 50;

  pub fn new(session_id: String, batch_num: usize) -> Self {
    Self {
      version: Self::VERSION,
      session_id,
      batch_num,
      ..Default::default()
    }
  }

  #[inline(always)]
  pub fn path(session_id: &str, batch: usize) -> Result<PathBuf, EngineError> {
    Ok(Self::join_path(&SessionV1::dir(session_id)?, batch))
  }

  #[inline(always)]
  pub fn join_path(dir: &Path, batch: usize) -> PathBuf {
    dir.join(format!("pages.{:03}.json", batch))
  }

  pub async fn load(session_id: String, batch: usize) -> Result<PageBatchV1, EngineError> {
    let path = Self::path(&session_id, batch)?;
    let mut res: PageBatchV1 = deserialize(&path).await?;
    res.session_id = session_id;
    res.batch_num = batch;
    Ok(res)
  }

  pub async fn save(&self) -> Result<(), EngineError> {
    let path = Self::path(&self.session_id, self.batch_num)?;
    serialize(&path, self).await?;
    Ok(())
  }

  /// Check whether this batch still has capacity for more pages.
  #[inline(always)]
  pub fn is_full(&self) -> bool {
    self.remain() > 0
  }

  /// Check whether this batch is currently completely empty.
  #[inline(always)]
  pub fn is_empty(&self) -> bool {
    self.pages.is_empty()
  }

  /// Count the number of remaining pages this batch can still hold.
  #[inline(always)]
  pub fn remain(&self) -> usize {
    Self::MAX_PAGES_PER_BATCH.saturating_sub(self.len())
  }

  /// Total number of pages in this batch.
  #[inline(always)]
  pub fn len(&self) -> usize {
    self.pages.len()
  }

  #[inline(always)]
  pub fn page_offset(page_index: usize) -> usize {
    page_index % Self::MAX_PAGES_PER_BATCH
  }

  #[inline(always)]
  pub fn batch_of(page_index: usize) -> usize {
    page_index / Self::MAX_PAGES_PER_BATCH
  }

  #[inline]
  pub fn parse_batch_idx(str: impl AsRef<str>) -> Option<usize> {
    let str = str.as_ref();
    let idx_str = str.trim_start_matches("pages.").trim_end_matches(".json");
    idx_str.parse::<usize>().ok()
  }

  pub fn to_conversation(&self) -> Conversation {
    let mut conv = Conversation::default();
    // We know that we will need AT LEAST pages.len() messages
    conv.messages.reserve(self.pages.len());
    for page in &self.pages {
      if let Some(system) = &page.system {
        conv.push(ConversationMessage::system(system.clone()));
      }
      if let Some(prompt) = &page.prompt {
        conv.push(ConversationMessage::user(prompt.clone()));
      }

      conv.messages.reserve(page.responses.len());
      for response in &page.responses {
        conv.push(ConversationMessage::assistant(
          response.content.as_ref().cloned().unwrap_or_default(),
          response.tool_calls.clone(),
        ));
        conv.messages.reserve(response.tool_responses.len());
        for (id, response) in &response.tool_responses {
          conv.push(ConversationMessage::tool(id.clone(), response.clone()));
        }
      }
    }
    conv
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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PageV1 {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub system: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub prompt: Option<String>,
  #[serde(skip_serializing_if = "Vec::is_empty")]
  pub responses: Vec<AgentResponseV1>,
}

impl PageV1 {
  /// Take the given slice of [ConversationMessage]s and convert them into
  /// [PageV1::responses].
  pub fn assimilate(&mut self, msgs: &[ConversationMessage]) -> Result<(), EngineError> {
    let mut iter = msgs.into_iter().peekable();
    let mut missing: HashSet<String> = HashSet::new();
    let mut result: Vec<AgentResponseV1> = vec![];
    while let Some(msg) = iter.next() {
      let ConversationMessage::Assistant { content, tool_calls } = msg else {
        return Err(EngineError::input("expected assistant message"));
      };
      let mut response = AgentResponseV1 {
        content: if content.trim() == "" {
          None
        } else {
          Some(content.clone())
        },
        tool_calls: tool_calls.clone(),
        tool_responses: vec![],
      };

      missing.extend(tool_calls.iter().map(|tc| tc.id.clone()));

      while let Some(msg) = iter.peek() && matches!(msg, ConversationMessage::Tool { .. }) {
        let Some(ConversationMessage::Tool { id, content }) = iter.next() else { unreachable!() };
        missing.remove(id);
        response.tool_responses.push((id.clone(), content.clone()));
      }

      if !missing.is_empty() {
        return Err(EngineError::input(format!("missing tool call responses for {}", itertools::join(missing, ", "))));
      }

      result.push(response);
    }

    self.responses = result;
    Ok(())
  }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentResponseV1 {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub content: Option<String>,
  #[serde(skip_serializing_if = "Vec::is_empty")]
  pub tool_calls: Vec<ToolCall>,
  #[serde(skip_serializing_if = "Vec::is_empty")]
  pub tool_responses: Vec<(String, String)>,
}
