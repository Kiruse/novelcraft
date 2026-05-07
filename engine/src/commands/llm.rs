use futures::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::OnceCell;

static MODELS: OnceCell<Mutex<HashMap<String, ModelConfig>>> = OnceCell::const_new();
static CLIENT: OnceCell<Client> = OnceCell::const_new();

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
  pub base_url: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub api_key: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct LlmMessage {
  pub author: String,
  pub content: String,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub tool_call_id: Option<String>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub tool_calls: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Deserialize)]
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

#[derive(Debug, Clone, Serialize)]
pub struct LlmUsage {
  pub prompt_tokens: u64,
  pub completion_tokens: u64,
  pub total_tokens: u64,
}

#[derive(Debug, Deserialize)]
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

fn emit_event(
  app: &AppHandle,
  base: &str,
  request_id: &Option<String>,
  payload: impl Serialize + Clone,
) {
  let event_name = match request_id {
    Some(id) => format!("{}:{}", base, id),
    None => base.to_string(),
  };
  let _ = app.emit(&event_name, payload);
}

fn models_config_path(app: &AppHandle) -> std::path::PathBuf {
  app
    .path()
    .app_data_dir()
    .unwrap_or_else(|_| std::path::PathBuf::from("."))
    .join("models.json")
}

pub async fn init_models(app: &AppHandle) {
  let path = models_config_path(app);
  let default_models: HashMap<String, ModelConfig> = HashMap::from([
    (
      "qwen/qwen3.5-9b".into(),
      ModelConfig {
        base_url: "http://localhost:1234/v1".into(),
        api_key: None,
      },
    ),
    (
      "zai-org/glm-4.6v-flash".into(),
      ModelConfig {
        base_url: "http://localhost:1234/v1".into(),
        api_key: None,
      },
    ),
  ]);

  let models = if path.exists() {
    let content = tokio::fs::read_to_string(&path).await.unwrap_or_default();
    serde_json::from_str(&content).unwrap_or(default_models)
  } else {
    default_models
  };

  MODELS.set(Mutex::new(models)).unwrap();
  CLIENT.set(Client::new()).unwrap();
}

#[tauri::command]
pub async fn prompt(app: AppHandle, request: LlmPromptRequest) -> Result<(), String> {
  let (client, config) = {
    let models = MODELS
      .get()
      .ok_or("Models not initialized")?
      .lock()
      .map_err(|e| e.to_string())?;

    let config = models
      .get(&request.model)
      .ok_or_else(|| format!("Unknown model: {}", request.model))?
      .clone();

    let client = CLIENT.get().ok_or("Client not initialized")?.clone();

    (client, config)
  };

  let mut api_messages: Vec<serde_json::Value> = Vec::new();

  if let Some(persona) = &request.persona {
    api_messages.push(serde_json::json!({
      "role": "system",
      "content": persona
    }));
  }

  for msg in &request.messages {
    let role = match msg.author.as_str() {
      "system" => "system",
      "user" => "user",
      "ai" => "assistant",
      other => other,
    };

    let mut api_msg = serde_json::json!({
      "role": role,
      "content": msg.content,
    });

    if role == "assistant" {
      if let Some(tool_calls) = &msg.tool_calls {
        if !tool_calls.is_empty() && msg.content.is_empty() {
          api_msg["content"] = serde_json::Value::Null;
        }
        api_msg["tool_calls"] = serde_json::json!(tool_calls);
      }
    }

    if let Some(tool_call_id) = &msg.tool_call_id {
      api_msg["tool_call_id"] = serde_json::json!(tool_call_id);
    }

    api_messages.push(api_msg);
  }

  let mut body = serde_json::json!({
    "model": request.model,
    "messages": api_messages,
    "stream": true,
    "stream_options": { "include_usage": true },
  });

  if let Some(tools) = &request.tools {
    let api_tools: Vec<serde_json::Value> = tools
      .iter()
      .map(|t| {
        let mut func = serde_json::json!({ "name": t.name });
        if let Some(desc) = &t.description {
          func["description"] = serde_json::json!(desc);
        }
        if let Some(params) = &t.parameters {
          func["parameters"] = params.clone();
        }
        serde_json::json!({ "type": "function", "function": func })
      })
      .collect();
    body["tools"] = serde_json::json!(api_tools);
  }

  let response = client
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
    .map_err(|e| format!("Request failed: {}", e))?;

  if !response.status().is_success() {
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    emit_event(&app, "llm:error", &request.request_id, &body);
    return Err(format!("LLM request failed: HTTP {}", status));
  }

  let mut stream = response.bytes_stream();
  let mut buffer = String::new();
  let mut finish_reason: Option<String> = None;
  let mut usage: Option<LlmUsage> = None;

  while let Some(chunk) = stream.next().await {
    let chunk = chunk.map_err(|e| format!("Stream error: {}", e))?;
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
          emit_event(
            &app,
            "llm:done",
            &request.request_id,
            LlmDonePayload {
              finish_reason: finish_reason.unwrap_or_else(|| "stop".to_string()),
              usage,
            },
          );
          return Ok(());
        }

        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(line) {
          if let Some(u) = parsed.get("usage").and_then(|u| u.as_object()) {
            usage = Some(LlmUsage {
              prompt_tokens: u
                .get("prompt_tokens")
                .and_then(|v| v.as_u64())
                .unwrap_or(0),
              completion_tokens: u
                .get("completion_tokens")
                .and_then(|v| v.as_u64())
                .unwrap_or(0),
              total_tokens: u
                .get("total_tokens")
                .and_then(|v| v.as_u64())
                .unwrap_or(0),
            });
          }

          if let Some(choices) = parsed.get("choices").and_then(|c| c.as_array()) {
            if let Some(choice) = choices.first() {
              if let Some(fr) = choice.get("finish_reason").and_then(|fr| fr.as_str()) {
                if !fr.is_empty() {
                  finish_reason = Some(fr.to_string());
                }
              }

              if let Some(delta) = choice.get("delta") {
                if let Some(content) = delta.get("content").and_then(|c| c.as_str()) {
                  if !content.is_empty() {
                    emit_event(&app, "llm:text", &request.request_id, content);
                  }
                }

                if let Some(reasoning) =
                  delta.get("reasoning_content").and_then(|r| r.as_str())
                {
                  if !reasoning.is_empty() {
                    emit_event(&app, "llm:reasoning", &request.request_id, reasoning);
                  }
                }

                if let Some(tool_calls) =
                  delta.get("tool_calls").and_then(|tc| tc.as_array())
                {
                  for tc in tool_calls {
                    let index =
                      tc.get("index").and_then(|i| i.as_u64()).unwrap_or(0) as u32;
                    let id = tc
                      .get("id")
                      .and_then(|i| i.as_str())
                      .map(String::from);
                    let name = tc
                      .get("function")
                      .and_then(|f| f.get("name"))
                      .and_then(|n| n.as_str())
                      .map(String::from);
                    let args_delta = tc
                      .get("function")
                      .and_then(|f| f.get("arguments"))
                      .and_then(|a| a.as_str())
                      .unwrap_or("");

                    emit_event(
                      &app,
                      "llm:tool_call",
                      &request.request_id,
                      LlmToolCallDelta {
                        index,
                        id,
                        name,
                        arguments_delta: args_delta.to_string(),
                      },
                    );
                  }
                }
              }
            }
          }
        }
      }
    }
  }

  emit_event(
    &app,
    "llm:done",
    &request.request_id,
    LlmDonePayload {
      finish_reason: finish_reason.unwrap_or_else(|| "stop".to_string()),
      usage,
    },
  );

  Ok(())
}

#[tauri::command]
pub async fn list_models(_app: AppHandle) -> Result<HashMap<String, ModelConfig>, String> {
  let models = MODELS
    .get()
    .ok_or("Models not initialized")?
    .lock()
    .map_err(|e| e.to_string())?;

  Ok(models.clone())
}

#[tauri::command]
pub async fn save_models(
  app: AppHandle,
  models: HashMap<String, ModelConfig>,
) -> Result<(), String> {
  let path = models_config_path(&app);
  if let Some(parent) = path.parent() {
    tokio::fs::create_dir_all(parent)
      .await
      .map_err(|e| e.to_string())?;
  }
  let content = serde_json::to_string_pretty(&models).map_err(|e| e.to_string())?;
  tokio::fs::write(&path, content)
    .await
    .map_err(|e| e.to_string())?;

  if let Some(stored) = MODELS.get() {
    let mut guard = stored.lock().map_err(|e| e.to_string())?;
    *guard = models;
  }

  Ok(())
}
