use reqwest::Client;
use serde::{Deserialize, Serialize};
use specta::Type;
use std::collections::HashMap;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::OnceCell;

use crate::error::AppError;
use crate::infer::api::*;
use crate::infer::internal::*;
use crate::util::{process_stream, StreamEvent};

static MODELS: OnceCell<Mutex<HashMap<String, ModelConfig>>> = OnceCell::const_new();
static CLIENT: OnceCell<Client> = OnceCell::const_new();

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
      "storyteller".into(),
      ModelConfig {
        base_url: "http://localhost:1234/v1".into(),
        model_id: "zai-org/glm-4.6v-flash".into(),
        api_key: None,
      },
    ),
    (
      "suggestions".into(),
      ModelConfig {
        base_url: "http://localhost:1234/v1".into(),
        model_id: "zai-org/glm-4.6v-flash".into(),
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
#[specta::specta]
pub async fn prompt(app: AppHandle, request: LlmPromptRequest) -> Result<(), AppError> {
  let (client, config) = {
    let models = MODELS
      .get()
      .ok_or(AppError::internal("Models not initialized"))?
      .lock()
      .map_err(|e| AppError::internal(e.to_string()))?;

    let config = models
      .get(&request.model)
      .ok_or_else(|| AppError::not_found(format!("Unknown model: {}", request.model)))?
      .clone();

    let client = CLIENT.get().ok_or(AppError::internal("Client not initialized"))?.clone();

    (client, config)
  };

  let mut api_messages: Vec<ApiChatMessage> = Vec::new();

  if let Some(persona) = &request.persona {
    api_messages.push(ApiChatMessage {
      role: "system".to_string(),
      content: Some(persona.clone()),
      tool_calls: None,
      tool_call_id: None,
    });
  }

  api_messages.extend(request.messages.iter().map(|msg| msg.clone().into()));

  let tools = request
    .tools
    .map(|tools| tools.into_iter().map(|t| t.into()).collect());

  let body = ChatCompletionRequest {
    model: config.model_id,
    messages: api_messages,
    stream: true,
    stream_options: StreamOptions {
      include_usage: true,
    },
    tools,
  };

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
    .map_err(|e| AppError::llm(format!("Request failed: {}", e)))?;

  if !response.status().is_success() {
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    emit_event(&app, "llm:error", &request.request_id, &body);
    return Err(AppError::llm(format!("LLM request failed: HTTP {}", status)));
  }

  let request_id = &request.request_id;

  process_stream(response.bytes_stream(), &|event| match event {
    StreamEvent::Text(text) => emit_event(&app, "llm:text", request_id, text.as_str()),
    StreamEvent::Reasoning(text) => emit_event(&app, "llm:reasoning", request_id, text.as_str()),
    StreamEvent::ToolCall {
      index,
      id,
      name,
      arguments_delta,
    } => emit_event(
      &app,
      "llm:tool_call",
      request_id,
      LlmToolCallDelta {
        index,
        id,
        name,
        arguments_delta,
      },
    ),
    StreamEvent::Done {
      finish_reason,
      usage,
    } => emit_event(
      &app,
      "llm:done",
      request_id,
      LlmDonePayload {
        finish_reason,
        usage,
      },
    ),
  })
  .await?;

  Ok(())
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct UnreachableHost {
  pub url: String,
  pub error: String,
}

#[tauri::command]
#[specta::specta]
pub async fn ping_hosts() -> Result<Vec<UnreachableHost>, AppError> {
  let unique_hosts: Vec<(String, String, Option<String>)> = {
    let models = MODELS
      .get()
      .ok_or(AppError::internal("Models not initialized"))?
      .lock()
      .map_err(|e| AppError::internal(e.to_string()))?;

    let mut seen = std::collections::HashSet::new();
    let mut hosts = Vec::new();

    for config in models.values() {
      if seen.insert(config.base_url.clone()) {
        hosts.push((
          config.base_url.clone(),
          config.model_id.clone(),
          config.api_key.clone(),
        ));
      }
    }

    hosts
  };

  let client = CLIENT.get().ok_or(AppError::internal("Client not initialized"))?.clone();

  let mut unreachable = Vec::new();

  for (base_url, _model_id, api_key) in &unique_hosts {
    let url = base_url.trim_end_matches('/');
    let mut req = client.get(format!("{}/models", url));
    if let Some(key) = api_key {
      req = req.header("Authorization", format!("Bearer {}", key));
    }
    let result = req.timeout(std::time::Duration::from_secs(5)).send().await;

    match result {
      Ok(resp) if resp.status().is_success() => {}
      Ok(resp) => {
        unreachable.push(UnreachableHost {
          url: base_url.clone(),
          error: format!("HTTP {}", resp.status()),
        });
      }
      Err(e) => {
        unreachable.push(UnreachableHost {
          url: base_url.clone(),
          error: e.to_string(),
        });
      }
    }
  }

  Ok(unreachable)
}

#[derive(Debug, Deserialize, Type)]
pub struct PingHostRequest {
  pub url: String,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub api_key: Option<String>,
}

#[tauri::command]
#[specta::specta]
pub async fn ping_host(request: PingHostRequest) -> Result<Option<String>, AppError> {
  let client = CLIENT.get().ok_or(AppError::internal("Client not initialized"))?.clone();

  let url = request.url.trim_end_matches('/');
  let mut req = client.get(format!("{}/models", url));
  if let Some(key) = &request.api_key {
    req = req.header("Authorization", format!("Bearer {}", key));
  }

  let result = req.timeout(std::time::Duration::from_secs(5)).send().await;

  match result {
    Ok(resp) if resp.status().is_success() => Ok(None),
    Ok(resp) => Ok(Some(format!("HTTP {}", resp.status()))),
    Err(e) => Ok(Some(e.to_string())),
  }
}

#[tauri::command]
#[specta::specta]
pub async fn list_models(_app: AppHandle) -> Result<HashMap<String, ModelConfig>, AppError> {
  let models = MODELS
    .get()
    .ok_or(AppError::internal("Models not initialized"))?
    .lock()
    .map_err(|e| AppError::internal(e.to_string()))?;

  Ok(models.clone())
}

#[tauri::command]
#[specta::specta]
pub async fn save_models(
  app: AppHandle,
  models: HashMap<String, ModelConfig>,
) -> Result<(), AppError> {
  let path = models_config_path(&app);
  if let Some(parent) = path.parent() {
    tokio::fs::create_dir_all(parent).await?;
  }
  let content = serde_json::to_string_pretty(&models)?;
  tokio::fs::write(&path, content).await?;

  if let Some(stored) = MODELS.get() {
    let mut guard = stored.lock().map_err(|e| AppError::internal(e.to_string()))?;
    *guard = models;
  }

  Ok(())
}
