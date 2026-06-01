use log::warn;
use serde::{Deserialize, Serialize};
use specta::Type;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::OnceCell;

use crate::error::AppError;
use crate::infer::openai::*;
use crate::infer::internal::*;
use crate::util::ensure_dir;
use crate::util::request_prompt;
use crate::util::reqwester;
use crate::util::{process_stream, StreamEvent};

const DEFAULT_HOST: &str = "http://localhost:1234/v1";
static MODELS: OnceCell<Mutex<Models>> = OnceCell::const_new();

#[derive(Debug, Copy, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all="snake_case")]
pub enum ModelUsage {
  Storyteller,
  Suggestions,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct Models {
  pub storyteller: ModelConfig,
  pub suggestions: ModelConfig,
}

impl Models {
  /// Patch this instance's `ModelConfig`s with empty `base_url`s to
  /// use the default host.
  pub fn patch(&mut self) {
    if self.storyteller.base_url.trim().is_empty() {
      self.storyteller.base_url = DEFAULT_HOST.to_string();
    }
    if self.suggestions.base_url.trim().is_empty() {
      self.suggestions.base_url = DEFAULT_HOST.to_string();
    }
  }

  pub async fn get(usage: ModelUsage) -> Result<ModelConfig, AppError> {
    let models = MODELS
      .get()
      .ok_or(AppError::internal("Models not initialized"))?
      .lock()
      .map_err(|e| AppError::internal(e.to_string()))?;
    Ok(match usage {
      ModelUsage::Storyteller => models.storyteller.clone(),
      ModelUsage::Suggestions => models.suggestions.clone(),
    })
  }

  async fn all() -> Result<Vec<ModelConfig>, AppError> {
    let models = MODELS
      .get()
      .ok_or(AppError::internal("Models not initialized"))?
      .lock()
      .map_err(|e| AppError::internal(e.to_string()))?;
    Ok(vec![
      models.storyteller.clone(),
      models.suggestions.clone(),
    ])
  }
}

impl Default for Models {
  fn default() -> Self {
    Self {
      storyteller: ModelConfig {
        base_url: DEFAULT_HOST.to_string(),
        model_id: "zai-org/glm-4.6v-flash".to_string(),
        api_key: None,
      },
      suggestions: ModelConfig {
        base_url: DEFAULT_HOST.to_string(),
        model_id: "zai-org/glm-4.6v-flash".to_string(),
        api_key: None,
      },
    }
  }
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ModelConfig {
  pub base_url: String,
  pub model_id: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub api_key: Option<String>,
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

  let models: Models = if path.exists() {
    let result = crate::util::deserialize::<Models>(&path).await;
    match result {
      Ok(models) => models,
      Err(err) => {
        warn!("Failed to deserialize models at {}: {} - initializing with defaults", path.to_string_lossy(), err);
        Models::default()
      }
    }
  } else {
    Models::default()
  };

  MODELS.set(Mutex::new(models)).unwrap();
}

#[tauri::command]
#[specta::specta]
pub async fn prompt(app: AppHandle, request: LlmPromptRequest) -> Result<(), AppError> {
  let client = reqwester().await;
  let config = Models::get(request.model).await?;
  let reqid = &request.request_id;

  let mut msgs: Vec<OpenAiChatMessage> = request.messages
    .iter()
    .map(|msg| msg.clone().into())
    .collect();

  if let Some(persona) = &request.persona {
    msgs.insert(0, OpenAiChatMessage::system(persona.clone()));
  }

  let tools = request
    .tools
    .map(|tools| tools.into_iter().map(|t| t.into()).collect());

  let response = request_prompt(&client, &config, msgs, tools).await?;

  if !response.status().is_success() {
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    emit_event(&app, "llm:error", &request.request_id, &body);
    return Err(AppError::llm(format!("LLM request failed: HTTP {}", status)));
  }

  let done = process_stream(response.bytes_stream(), &mut |event| match event {
    StreamEvent::Text(text) => emit_event(&app, "llm:text", reqid, text.as_str()),
    StreamEvent::Reasoning(text) => emit_event(&app, "llm:reasoning", reqid, text.as_str()),
    StreamEvent::ToolCall {
      index,
      id,
      name,
      arguments_delta,
    } => emit_event(
      &app,
      "llm:tool_call",
      reqid,
      LlmToolCallDelta {
        index,
        id,
        name,
        arguments_delta,
      },
    ),
  }).await?;

  emit_event(
    &app,
    "llm:done",
    reqid,
    LlmDonePayload::from(done),
  );

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
  let models = Models::all().await?;
  let hosts = models
    .iter()
    .map(|m| (&m.base_url, &m.api_key))
    .collect::<Vec<_>>();

  let client = reqwester().await;
  let mut unreachable = Vec::new();

  for (base_url, api_key) in hosts {
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
  let client = reqwester().await;
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
pub async fn list_models() -> Result<Models, AppError> {
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
  mut models: Models,
) -> Result<(), AppError> {
  models.patch();

  let path = models_config_path(&app);
  ensure_dir(&path).await?;
  crate::util::serialize(&path, &models).await?;

  let mut guard = MODELS
    .get()
    .ok_or(AppError::internal("Models not initialized"))?
    .lock()
    .map_err(|e| AppError::internal(e.to_string()))?;
  *guard = models.clone();

  Ok(())
}
