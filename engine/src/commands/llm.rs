use std::path::PathBuf;

use log::warn;
use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::game::state::AppState;
use crate::infer::openai::*;
use crate::infer::internal::*;
use crate::commands::paths;
use crate::util::ensure_dir;
use crate::util::request_prompt;
use crate::util::reqwester;
use crate::util::{process_stream, StreamEvent};

const DEFAULT_HOST: &str = "http://localhost:1234/v1";

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
#[serde(rename_all="snake_case")]
pub enum ModelUsage {
  Storyteller,
  Suggestions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Models {
  pub storyteller: ModelConfig,
  pub suggestions: ModelConfig,
}

impl Models {
  pub fn patch(&mut self) {
    if self.storyteller.base_url.trim().is_empty() {
      self.storyteller.base_url = DEFAULT_HOST.to_string();
    }
    if self.suggestions.base_url.trim().is_empty() {
      self.suggestions.base_url = DEFAULT_HOST.to_string();
    }
  }

  pub fn get_config(&self, usage: ModelUsage) -> ModelConfig {
    match usage {
      ModelUsage::Storyteller => self.storyteller.clone(),
      ModelUsage::Suggestions => self.suggestions.clone(),
    }
  }

  pub fn all_configs(&self) -> Vec<ModelConfig> {
    vec![self.storyteller.clone(), self.suggestions.clone()]
  }

  pub async fn load() -> Result<Models, AppError> {
    let path = Self::config_path()?;
    if path.exists() {
      let result = crate::util::deserialize::<Models>(&path).await;
      match result {
        Ok(mut models) => {
          models.patch();
          Ok(models)
        }
        Err(err) => {
          warn!("Failed to deserialize models at {}: {} - initializing with defaults", path.to_string_lossy(), err);
          Ok(Models::default())
        }
      }
    } else {
      Ok(Models::default())
    }
  }

  pub async fn save(&self) -> Result<(), AppError> {
    let path = Self::config_path()?;
    ensure_dir(&path).await?;
    crate::util::serialize(&path, self).await
  }

  fn config_path() -> Result<PathBuf, AppError> {
    paths::config_dir().map(|p: PathBuf| p.join("models.json"))
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
  pub base_url: String,
  pub model_id: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub api_key: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UnreachableHost {
  pub url: String,
  pub error: String,
}

#[derive(Debug, Deserialize)]
pub struct PingHostRequest {
  pub url: String,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub api_key: Option<String>,
}

pub async fn prompt(
  state: &AppState,
  request: LlmPromptRequest,
  on_text: impl Fn(String),
  on_reasoning: impl Fn(String),
  on_tool_call: impl Fn(LlmToolCallDelta),
  on_done: impl Fn(LlmDonePayload),
  on_error: impl Fn(String),
) -> Result<(), AppError> {
  let client = reqwester().await;
  let config = state.models.lock().await.get_config(request.model);

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
    on_error(body);
    return Err(AppError::llm(format!("LLM request failed: HTTP {}", status)));
  }

  let done = process_stream(response.bytes_stream(), &mut |event| match event {
    StreamEvent::Text(text) => on_text(text),
    StreamEvent::Reasoning(text) => on_reasoning(text),
    StreamEvent::ToolCall {
      index,
      id,
      name,
      arguments_delta,
    } => on_tool_call(LlmToolCallDelta {
      index,
      id,
      name,
      arguments_delta,
    }),
  }).await?;

  on_done(LlmDonePayload::from(done));

  Ok(())
}

pub async fn ping_hosts(state: &AppState) -> Result<Vec<UnreachableHost>, AppError> {
  let models = state.models.lock().await.all_configs();
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
