use std::str::FromStr;

use crate::error::AppError;

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

impl From<internal::ToolCall> for ToolCall {
  fn from(value: internal::ToolCall) -> Self {
    Self {
      id: value.id,
      r#type: "function".to_string(),
      function: FunctionCall {
        name: value.name,
        arguments: value.arguments,
      },
    }
  }
}

#[derive(Debug, Clone, Serialize)]
pub struct OpenAiChatMessage {
  role: OpenAiChatRole,
  #[serde(skip_serializing_if = "Option::is_none")]
  content: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  tool_calls: Option<Vec<ToolCall>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  tool_call_id: Option<String>,
}

impl OpenAiChatMessage {
  pub fn system(content: String) -> Self {
    Self {
      role: OpenAiChatRole::System,
      content: Some(content),
      ..Default::default()
    }
  }

  pub fn ai(content: String) -> Self {
    Self {
      role: OpenAiChatRole::Assistant,
      content: Some(content),
      ..Default::default()
    }
  }

  pub fn user(content: String) -> Self {
    Self {
      role: OpenAiChatRole::User,
      content: Some(content),
      ..Default::default()
    }
  }

  pub fn toolcall(content: Option<String>, tool_calls: Vec<ToolCall>) -> Self {
    Self {
      role: OpenAiChatRole::Assistant,
      content,
      tool_call_id: None,
      tool_calls: Some(tool_calls),
    }
  }

  pub fn toolresult(id: String, content: Result<String, String>) -> Self {
    Self {
      role: OpenAiChatRole::Tool,
      content: match content {
        Ok(msg) => Some(msg),
        Err(msg) => Some(format!("Error: {}", msg).to_string()),
      },
      tool_call_id: Some(id),
      ..Default::default()
    }
  }
}

impl Default for OpenAiChatMessage {
  fn default() -> Self {
    Self {
      role: OpenAiChatRole::User,
      content: None,
      tool_call_id: None,
      tool_calls: None,
    }
  }
}

impl From<internal::LlmMessage> for OpenAiChatMessage {
  fn from(value: internal::LlmMessage) -> Self {
    Self {
      role: value.author.parse().unwrap_or_default(),
      content: if value.content.is_empty() {
        None
      } else {
        Some(value.content)
      },
      tool_call_id: value.tool_call_id,
      tool_calls: value.tool_calls.and_then(|toolcalls| Some(toolcalls
        .iter()
        .map(|tc| super::openai::ToolCall {
          id: tc.id.clone(),
          r#type: "function".to_string(),
          function: super::openai::FunctionCall {
            name: tc.name.clone(),
            arguments: tc.arguments.clone(),
          },
        })
        .collect::<Vec<_>>())),
    }
  }
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OpenAiChatRole {
  #[default]
  User,
  Assistant,
  System,
  Tool,
}

impl ToString for OpenAiChatRole {
  fn to_string(&self) -> String {
    match self {
      Self::User => "user".to_string(),
      Self::Assistant => "assistant".to_string(),
      Self::System => "system".to_string(),
      Self::Tool => "tool".to_string(),
    }
  }
}

impl FromStr for OpenAiChatRole {
  type Err = AppError;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    match s {
      "system"      => Ok(Self::System),
      "assistant"   => Ok(Self::Assistant),
      "ai"          => Ok(Self::Assistant),
      "user"        => Ok(Self::User),
      "tool"        => Ok(Self::Tool),
      "tool_result" => Ok(Self::Tool),
      "tool-result" => Ok(Self::Tool),
      _ => Err(AppError::parse(format!("unknown chat role {}", s))),
    }
  }
}

#[derive(Debug, Clone, Serialize)]
pub struct OpenAiToolFunction {
  pub name: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub description: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub parameters: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize)]
pub struct OpenAiTool {
  pub r#type: &'static str,
  pub function: OpenAiToolFunction,
}

impl From<internal::LlmTool> for OpenAiTool {
  fn from(value: internal::LlmTool) -> Self {
    Self {
      r#type: "function",
      function: OpenAiToolFunction {
        name: value.name,
        description: value.description,
        parameters: value.parameters,
      },
    }
  }
}

#[derive(Debug, Clone, Serialize)]
pub struct OpenAiStreamOptions {
  pub include_usage: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct OpenAiCompletionRequest {
  pub model: String,
  pub messages: Vec<OpenAiChatMessage>,
  pub stream: bool,
  pub stream_options: OpenAiStreamOptions,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub tools: Option<Vec<OpenAiTool>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OpenAiStreamResponse {
  pub usage: Option<OpenAiStreamUsage>,
  pub choices: Option<Vec<OpenAiStreamChoice>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAiStreamUsage {
  pub prompt_tokens: u64,
  pub completion_tokens: u64,
  pub total_tokens: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OpenAiStreamChoice {
  pub finish_reason: Option<String>,
  pub delta: Option<OpenAiStreamDelta>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OpenAiStreamDelta {
  pub content: Option<String>,
  pub reasoning_content: Option<String>,
  pub tool_calls: Option<Vec<OpenAiStreamToolCall>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OpenAiStreamToolCall {
  pub index: u64,
  pub id: Option<String>,
  pub function: OpenAiStreamFunctionDelta,
}

impl From<OpenAiStreamToolCall> for internal::LlmToolCallDelta {
  fn from(value: OpenAiStreamToolCall) -> Self {
    Self {
      id: value.id,
      index: value.index as u32,
      name: value.function.name,
      arguments_delta: value.function.arguments.unwrap_or_default(),
    }
  }
}

#[derive(Debug, Clone, Deserialize)]
pub struct OpenAiStreamFunctionDelta {
  pub name: Option<String>,
  pub arguments: Option<String>,
}
