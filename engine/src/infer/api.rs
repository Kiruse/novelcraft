use super::internal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
  pub name: String,
  pub arguments: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
  pub id: String,
  pub r#type: String,
  pub function: FunctionCall,
}

#[derive(Debug, Clone, Serialize)]
pub struct ApiChatMessage {
  pub role: String,
  pub content: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub tool_calls: Option<Vec<ToolCall>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub tool_call_id: Option<String>,
}

impl From<internal::LlmMessage> for ApiChatMessage {
  fn from(value: internal::LlmMessage) -> Self {
    Self {
      role: match value.author.as_str() {
        "ai" => "assistant",
        other => other,
      }
      .to_string(),
      content: if value.content.is_empty() {
        None
      } else {
        Some(value.content)
      },
      tool_call_id: value.tool_call_id,
      tool_calls: value.tool_calls,
    }
  }
}

#[derive(Debug, Clone, Serialize)]
pub struct ApiToolFunction {
  pub name: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub description: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub parameters: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ApiTool {
  pub r#type: &'static str,
  pub function: ApiToolFunction,
}

impl From<internal::LlmTool> for ApiTool {
  fn from(value: internal::LlmTool) -> Self {
    Self {
      r#type: "function",
      function: ApiToolFunction {
        name: value.name,
        description: value.description,
        parameters: value.parameters,
      },
    }
  }
}

#[derive(Debug, Clone, Serialize)]
pub struct StreamOptions {
  pub include_usage: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChatCompletionRequest {
  pub model: String,
  pub messages: Vec<ApiChatMessage>,
  pub stream: bool,
  pub stream_options: StreamOptions,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub tools: Option<Vec<ApiTool>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StreamResponse {
  pub usage: Option<StreamUsage>,
  pub choices: Option<Vec<StreamChoice>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamUsage {
  pub prompt_tokens: u64,
  pub completion_tokens: u64,
  pub total_tokens: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StreamChoice {
  pub finish_reason: Option<String>,
  pub delta: Option<StreamDelta>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StreamDelta {
  pub content: Option<String>,
  pub reasoning_content: Option<String>,
  pub tool_calls: Option<Vec<StreamToolCall>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StreamToolCall {
  pub index: u64,
  pub id: Option<String>,
  pub function: StreamFunctionDelta,
}

impl From<StreamToolCall> for internal::LlmToolCallDelta {
  fn from(value: StreamToolCall) -> Self {
    Self {
      id: value.id,
      index: value.index as u32,
      name: value.function.name,
      arguments_delta: value.function.arguments.unwrap_or_default(),
    }
  }
}

#[derive(Debug, Clone, Deserialize)]
pub struct StreamFunctionDelta {
  pub name: Option<String>,
  pub arguments: Option<String>,
}
