use std::path::Path;

use crate::error::AppError;
use crate::infer::openai::{OpenAiChatMessage, OpenAiTool, OpenAiCompletionRequest, OpenAiStreamOptions, OpenAiStreamResponse, OpenAiStreamUsage};
use chrono::{DateTime, Utc};
use futures::StreamExt;
use reqwest::{Client, Response};
use serde::{Deserialize, Deserializer, Serializer};
use serde::{Serialize, de::DeserializeOwned};
use tokio::sync::OnceCell;

const CLIENT: OnceCell<Client> = OnceCell::const_new();

pub async fn reqwester() -> Client {
  CLIENT.get_or_init(async || Client::new()).await.clone()
}

pub enum StreamEvent {
  Text(String),
  Reasoning(String),
  ToolCall {
    index: u32,
    id: Option<String>,
    name: Option<String>,
    arguments_delta: String,
  },
}

#[derive(Debug, Clone)]
pub struct StreamDone {
  pub finish_reason: String,
  pub usage: Option<OpenAiStreamUsage>,
}

pub async fn process_stream<S, F>(stream: S, on_event: &mut F) -> Result<StreamDone, AppError>
where
  S: futures::Stream<Item = Result<bytes::Bytes, reqwest::Error>> + Unpin,
  F: FnMut(StreamEvent),
{
  let mut buffer = String::new();
  let mut finish_reason = "stop".to_string();
  let mut usage: Option<OpenAiStreamUsage> = None;

  let mut stream = Box::pin(stream);

  while let Some(chunk) = stream.next().await {
    let chunk = chunk.map_err(|e| AppError::llm(format!("Stream error: {}", e)))?;
    buffer.push_str(&String::from_utf8_lossy(&chunk));

    while let Some(pos) = buffer.find("\n\n") {
      let frame = buffer[..pos].to_string();
      buffer = buffer[pos + 2..].to_string();

      for line in frame.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with(':') {
          continue;
        }

        let line = line.strip_prefix("data: ").unwrap_or(line);
        if line == "[DONE]" {
          return Ok(StreamDone { finish_reason, usage });
        }

        if let Ok(parsed) = serde_json::from_str::<OpenAiStreamResponse>(line) {
          if let Some(u) = parsed.usage {
            usage = Some(u);
          }

          if let Some(choice) = parsed.choices.as_ref().and_then(|choices| choices.first()) {
            if let Some(fr) = &choice.finish_reason {
              if !fr.is_empty() {
                finish_reason = fr.clone();
              }
            }

            if let Some(delta) = &choice.delta {
              if let Some(content) = &delta.content {
                if !content.is_empty() {
                  on_event(StreamEvent::Text(content.clone()));
                }
              }

              if let Some(reasoning) = &delta.reasoning_content {
                if !reasoning.is_empty() {
                  on_event(StreamEvent::Reasoning(reasoning.clone()));
                }
              }

              if let Some(tool_calls) = &delta.tool_calls {
                for tc in tool_calls {
                  on_event(StreamEvent::ToolCall {
                    index: tc.index as u32,
                    id: tc.id.clone(),
                    name: tc.function.name.clone(),
                    arguments_delta: tc.function.arguments.clone().unwrap_or_default(),
                  });
                }
              }
            }
          }
        }
      }
    }
  }

  Ok(StreamDone { finish_reason, usage })
}

pub async fn ensure_dir(path: &Path) -> Result<(), AppError> {
  tokio::fs::create_dir_all(path)
    .await
    .map_err(|e| AppError::io(format!("Failed to create directory: {}", e)))
}

pub async fn deserialize<T: DeserializeOwned>(path: &Path) -> Result<T, AppError> {
  let raw = tokio::fs::read_to_string(path)
    .await
    .map_err(|e| AppError::io(format!("Read error: {}", e)))?;
  serde_json::from_str(&raw).map_err(|e| AppError::parse(format!("Parse error: {}", e)))
}

pub async fn serialize(path: &Path, obj: &impl Serialize) -> Result<(), AppError> {
  if let Some(parent) = path.parent() {
    ensure_dir(parent).await?;
  }
  let content =
    serde_json::to_string_pretty(obj).map_err(|e| AppError::parse(format!("Serialize error: {}", e)))?;
  tokio::fs::write(path, content)
    .await
    .map_err(|e| AppError::io(format!("Write error: {}", e)))
}

pub async fn request_prompt(
  client: &Client,
  config: &crate::commands::llm::ModelConfig,
  messages: Vec<OpenAiChatMessage>,
  tools: Option<Vec<OpenAiTool>>,
) -> Result<Response, AppError> {
  let body = OpenAiCompletionRequest {
    model: config.model_id.clone(),
    messages,
    stream: true,
    stream_options: OpenAiStreamOptions {
      include_usage: true,
    },
    tools,
  };

  Ok(client
    .post(format!("{}/chat/completions", config.base_url))
    .header("Content-Type", "application/json")
    .header(
      "Authorization",
      match &config.api_key {
        Some(key) => format!("Bearer {}", key),
        None => "Bearer no-key".to_string(),
      },
    )
    .json(&body)
    .send()
    .await
    .map_err(|e| AppError::llm(format!("Request failed: {}", e)))?)
}

pub fn serialize_timestamp<S>(value: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error>
where S: Serializer
{
  serializer.serialize_str(&value.to_rfc3339())
}

pub fn deserialize_timestamp<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
where D: Deserializer<'de>
{
  let s = String::deserialize(deserializer)?;
  Ok(DateTime::parse_from_rfc3339(&s)
    .map_err(|e| serde::de::Error::custom(e.to_string()))?
    .to_utc())
}
