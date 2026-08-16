use serde::{Deserialize, Serialize};

use crate::commands::llm::ModelUsage;
use crate::util::StreamDone;

pub type LlmUsage = super::openai::OpenAiStreamUsage;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ToolCall {
  pub id: String,
  pub name: String,
  pub arguments: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmMessage {
  pub author: String,
  pub content: String,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub tool_call_id: Option<String>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub tool_calls: Option<Vec<ToolCall>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmTool {
  pub name: String,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub description: Option<String>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub parameters: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmToolCallDelta {
  pub index: u32,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub id: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub name: Option<String>,
  pub arguments_delta: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmDonePayload {
  pub finish_reason: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub usage: Option<LlmUsage>,
}

impl From<StreamDone> for LlmDonePayload {
  fn from(value: StreamDone) -> Self {
    Self {
      finish_reason: value.finish_reason,
      usage: value.usage,
    }
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmPromptRequest {
  pub model: ModelUsage,
  pub messages: Vec<LlmMessage>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub persona: Option<String>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  #[allow(dead_code)]
  pub context: Option<serde_json::Value>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub request_id: Option<String>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub tools: Option<Vec<LlmTool>>,
}
