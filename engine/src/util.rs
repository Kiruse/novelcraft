use std::path::{Path, PathBuf};

use crate::error::AppError;
use crate::infer::api::{StreamResponse, StreamUsage};
use futures::StreamExt;
use serde::{Serialize, de::DeserializeOwned};

pub enum StreamEvent {
  Text(String),
  Reasoning(String),
  ToolCall {
    index: u32,
    id: Option<String>,
    name: Option<String>,
    arguments_delta: String,
  },
  Done {
    finish_reason: String,
    usage: Option<StreamUsage>,
  },
}

pub async fn process_stream<S, F>(stream: S, on_event: &F) -> Result<(), AppError>
where
  S: futures::Stream<Item = Result<bytes::Bytes, reqwest::Error>> + Unpin,
  F: Fn(StreamEvent),
{
  let mut buffer = String::new();
  let mut finish_reason: Option<String> = None;
  let mut usage: Option<StreamUsage> = None;

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
          on_event(StreamEvent::Done {
            finish_reason: finish_reason.unwrap_or_else(|| "stop".to_string()),
            usage,
          });
          return Ok(());
        }

        if let Ok(parsed) = serde_json::from_str::<StreamResponse>(line) {
          if let Some(u) = parsed.usage {
            usage = Some(u);
          }

          if let Some(choice) = parsed.choices.as_ref().and_then(|choices| choices.first()) {
            if let Some(fr) = &choice.finish_reason {
              if !fr.is_empty() {
                finish_reason = Some(fr.clone());
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

  on_event(StreamEvent::Done {
    finish_reason: finish_reason.unwrap_or_else(|| "stop".to_string()),
    usage,
  });

  Ok(())
}

pub fn canonical_path(path: &Path) -> Result<PathBuf, AppError> {
  std::fs::canonicalize(path).map_err(|e| AppError::io(e.to_string()))
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
