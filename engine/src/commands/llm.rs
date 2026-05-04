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
}

#[derive(Debug, Deserialize)]
pub struct LlmPromptRequest {
  pub model: String,
  pub messages: Vec<LlmMessage>,
  pub persona: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  #[allow(dead_code)]
  pub context: Option<serde_json::Value>,
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
  api_messages.push(serde_json::json!({
    "role": "system",
    "content": request.persona
  }));

  for msg in &request.messages {
    let role = match msg.author.as_str() {
      "system" => "system",
      "user" => "user",
      "ai" => "assistant",
      other => other,
    };
    api_messages.push(serde_json::json!({
      "role": role,
      "content": msg.content
    }));
  }

  let body = serde_json::json!({
    "model": request.model,
    "messages": api_messages,
    "stream": true,
  });

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
    let _ = app.emit("llm:error", body);
    return Err(format!("LLM request failed: HTTP {}", status));
  }

  let mut stream = response.bytes_stream();
  let mut buffer = String::new();

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
          let _ = app.emit("llm:done", "");
          return Ok(());
        }

        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(line) {
          if let Some(choices) = parsed.get("choices").and_then(|c| c.as_array()) {
            if let Some(choice) = choices.first() {
              if let Some(delta) = choice.get("delta") {
                if let Some(content) = delta.get("content").and_then(|c| c.as_str()) {
                  if !content.is_empty() {
                    let _ = app.emit("llm:text", content);
                  }
                }
                if let Some(reasoning) = delta.get("reasoning_content").and_then(|r| r.as_str()) {
                  if !reasoning.is_empty() {
                    let _ = app.emit("llm:reasoning", reasoning);
                  }
                }
              }
            }
          }
        }
      }
    }
  }

  let _ = app.emit("llm:done", "");
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
