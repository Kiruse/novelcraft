use std::collections::HashMap;

use log::warn;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::commands::llm::{ModelConfig, ModelUsage};
use crate::error::AppError;
use crate::game::engine::GameEngine;
use crate::game::pages::{PageBatch, PageV1};
use crate::game::session::SessionV1;
use crate::game::state::AppState;
use crate::infer::openai::{OpenAiChatMessage, OpenAiTool};
use crate::infer::internal::{LlmUsage, ToolCall};
use crate::util::{StreamDone, StreamEvent, process_stream, request_prompt, reqwester};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GamePromptResult {
  pub stream_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GamePromptEvent {
  Reasoning {
    step: u8,
    delta: String,
  },
  Text {
    step: u8,
    delta: String,
  },
  ToolCalls {
    step: u8,
    text: Option<String>,
    calls: Vec<ToolCall>,
  },
  Done {
    finish_reason: String,
    usage: Option<LlmUsage>,
  },
  Error(GamePromptError),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "domain")]
#[serde(rename_all = "snake_case")]
pub enum GamePromptError {
  Request {
    status: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    body: Option<String>,
  },
  Internal(AppError),
}

#[derive(Debug, Clone)]
pub struct AgentLoopResult {
  pub finish_reason: String,
  pub usage: Option<LlmUsage>,
}

impl Default for AgentLoopResult {
  fn default() -> Self {
    Self {
      finish_reason: "error".to_string(),
      usage: None,
    }
  }
}

#[derive(Debug, Clone)]
enum StepResult {
  ToolCalls {
    text: Option<String>,
    toolcalls: Vec<ToolCall>,
  },
  Final(String, StreamDone),
}

pub async fn game_sessions() -> Result<Vec<SessionV1>, AppError> {
  Ok(SessionV1::list().await?)
}

pub async fn game_page(session_id: &str, page: u32) -> Result<PageV1, AppError> {
  let engine = GameEngine::from_session_id(session_id).await?;
  Ok(engine.page(page as usize).await?)
}

pub async fn game_prompt<F>(
  state: &AppState,
  session_id: &str,
  prompt: String,
  on_event: F,
) -> Result<GamePromptResult, AppError>
where
  F: Fn(GamePromptEvent) + Send + Sync + 'static,
{
  let stream_id = uuid::Uuid::new_v4().to_string();
  let prompt = prompt.trim().to_string();
  let res = GamePromptResult { stream_id: stream_id.clone() };
  let state = state.clone();
  let session_id = session_id.to_string();

  tokio::spawn(async move {
    if let Err(e) = agent_loop(&state, &session_id, &stream_id, prompt, &on_event).await {
      warn!("Error in agent loop: {}", e);
      on_event(GamePromptEvent::Error(GamePromptError::Internal(e)));
    }
  });

  Ok(res)
}

pub async fn game_fork<F>(
  state: &AppState,
  session_id: &str,
  page_index: u32,
  prompt: String,
  on_event: F,
) -> Result<GamePromptResult, AppError>
where
  F: Fn(GamePromptEvent) + Send + Sync + 'static,
{
  let stream_id = uuid::Uuid::new_v4().to_string();
  let prompt = prompt.trim().to_string();
  let res = GamePromptResult { stream_id: stream_id.clone() };
  let state = state.clone();
  let session_id = session_id.to_string();

  tokio::spawn(async move {
    if let Err(e) = fork_internal(&state, &session_id, &stream_id, page_index as usize, prompt, &on_event).await {
      warn!("Error in agent loop: {}", e);
      on_event(GamePromptEvent::Error(GamePromptError::Internal(e)));
    }
  });

  Ok(res)
}

async fn fork_internal<F>(
  state: &AppState,
  session_id: &String,
  stream_id: &String,
  page_index: usize,
  prompt: String,
  on_event: &F,
) -> Result<(), AppError>
where
  F: Fn(GamePromptEvent) + Send + Sync,
{
  GameEngine::from_session_id(session_id).await?.fork(page_index).await?;
  agent_loop(state, session_id, stream_id, prompt, on_event).await?;
  Ok(())
}

async fn agent_loop<F>(
  state: &AppState,
  session_id: &String,
  stream_id: &String,
  prompt: String,
  on_event: &F,
) -> Result<AgentLoopResult, AppError>
where
  F: Fn(GamePromptEvent) + Send + Sync,
{
  let engine = GameEngine::from_session_id(session_id).await?;
  let history = engine.history(PageBatch::MAX_PAGES_PER_BATCH)?;
  let mut msgs: Vec<OpenAiChatMessage> = PageV1::to_openai_messages(&history);
  let mut step_idx = 0u8;

  let max_steps = state.config.lock().await.max_agent_steps;
  let config = state.models.lock().await.get_config(ModelUsage::Storyteller);

  let client = reqwester().await;

  if !prompt.is_empty() {
    msgs.push(OpenAiChatMessage::user(prompt));
  }

  let mut done = false;
  let mut result = AgentLoopResult::default();
  while !done && step_idx < max_steps {
    let step_result = step(
      &client,
      &config,
      &msgs,
      step_idx,
      stream_id,
      on_event,
    ).await?;

    match step_result {
      StepResult::ToolCalls { text, toolcalls } => {
        msgs.push(OpenAiChatMessage::toolcall(
          text,
          toolcalls
            .iter()
            .map(|tc| tc.clone().into())
            .collect::<Vec<_>>(),
        ));
        for _toolcall in toolcalls.iter() {
          todo!("implement actual tool calling");
        }
      }
      StepResult::Final(text, stream_done) => {
        if text.is_empty() {
          return Err(AppError::llm("unexpected empty response"));
        }
        msgs.push(OpenAiChatMessage::ai(text));
        result.finish_reason = stream_done.finish_reason;
        result.usage = stream_done.usage;
        done = true;
      }
    }

    step_idx += 1;
  }

  if &result.finish_reason == "error" {
    Err(AppError::internal("agent loop failed to converge"))
  } else {
    on_event(GamePromptEvent::Done {
      finish_reason: result.finish_reason.clone(),
      usage: result.usage.clone(),
    });
    Ok(result)
  }
}

async fn step<F>(
  client: &Client,
  config: &ModelConfig,
  msgs: &Vec<OpenAiChatMessage>,
  step_idx: u8,
  _stream_id: &String,
  on_event: &F,
) -> Result<StepResult, AppError>
where
  F: Fn(GamePromptEvent) + Send + Sync,
{
  let tools: Vec<OpenAiTool> = vec![];

  let response = request_prompt(&client, &config, msgs.clone(), Some(tools)).await?;

  if !response.status().is_success() {
    let status = response.status().as_u16();
    let body = response.text().await.ok();
    on_event(GamePromptEvent::Error(GamePromptError::Request { status, body: body.clone() }));
    return Err(AppError::llm(format!("LLM request failed with status {}, message: {}", status, body.unwrap_or("None".to_string()))));
  }

  let mut text: String = String::new();
  let mut toolcalls: HashMap<u32, ToolCall> = HashMap::new();

  let done = process_stream(response.bytes_stream(), &mut |event| match event {
    StreamEvent::Reasoning(delta) => {
      if !delta.is_empty() {
        on_event(GamePromptEvent::Reasoning { step: step_idx, delta });
      }
    }
    StreamEvent::Text(delta) => {
      if !delta.is_empty() {
        text += &delta;
        on_event(GamePromptEvent::Text { step: step_idx, delta });
      }
    }
    StreamEvent::ToolCall { index, id, name, arguments_delta } => {
      if let Some(val) = toolcalls.get_mut(&index) {
        val.arguments += &arguments_delta;
      } else {
        toolcalls.insert(index, ToolCall {
          id: id.unwrap_or_default(),
          name: name.unwrap_or_default(),
          arguments: arguments_delta,
        });
      }
    }
  }).await?;

  let mut toolcalls = toolcalls.into_iter().collect::<Vec<_>>();
  toolcalls.sort_by_key(|(key, _)| *key);
  let toolcalls = toolcalls.into_iter().map(|(_, value)| value).collect::<Vec<_>>();

  if !toolcalls.is_empty() {
    let text = if text.trim().is_empty() {
      None
    } else {
      Some(text.trim().to_string())
    };

    on_event(GamePromptEvent::ToolCalls {
      step: step_idx,
      text: text.clone(),
      calls: toolcalls.clone(),
    });

    Ok(StepResult::ToolCalls {
      text,
      toolcalls,
    })
  } else {
    Ok(StepResult::Final(text, done))
  }
}
