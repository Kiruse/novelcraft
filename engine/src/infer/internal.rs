use serde::{Deserialize, Serialize};

pub type ToolCall = super::api::ToolCall;
pub type LlmUsage = super::api::StreamUsage;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
  pub base_url: String,
  pub model_id: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub api_key: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LlmMessage {
  pub author: String,
  pub content: String,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub tool_call_id: Option<String>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub tool_calls: Option<Vec<ToolCall>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LlmTool {
  pub name: String,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub description: Option<String>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub parameters: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LlmToolCallDelta {
  pub index: u32,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub id: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub name: Option<String>,
  pub arguments_delta: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct LlmDonePayload {
  pub finish_reason: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub usage: Option<LlmUsage>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LlmPromptRequest {
  pub model: String,
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
