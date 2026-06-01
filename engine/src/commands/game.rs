use std::collections::HashMap;

use log::{warn};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{AppHandle, Emitter};
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::commands::llm::{ModelConfig, ModelUsage, Models};
use crate::config::NovelCraftConfig;
use crate::error::AppError;
use crate::game::engine::GameEngine;
use crate::game::pages::{PageBatch, PageV1};
use crate::game::session::SessionV1;
use crate::infer::openai::{OpenAiChatMessage, OpenAiTool};
use crate::infer::internal::{LlmUsage, ToolCall};
use crate::util::{StreamDone, StreamEvent, process_stream, request_prompt, reqwester};

static MAX_STEPS: Mutex<u8> = Mutex::const_new(10);

const PROMPT_WRITE_MORE: &str = "Please continue the narration.";

const PROMPT_INSTRUCT: &str = "The player has given instructions for how to rewrite the above page of this interactive \
story. Follow their instructions — you may make substantial or minimal changes as requested. \
Your response will replace the previous page.";

const PROMPT_STEER: &str = "The player wants to adjust the direction of this interactive story while keeping the same \
general events and narrative voice. Rewrite the above passage incorporating the player's guidance. \
Your response will replace the previous page.";

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all="snake_case")]
pub enum GamePromptMode {
  Write,
  Instruct,
  Steer,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct GamePromptResult {
  pub stream_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
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

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
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

async fn get_max_steps() -> u8 {
  *MAX_STEPS.lock().await
}

pub async fn init_engine(_app: &AppHandle, cfg: &NovelCraftConfig) {
  let mut guard = MAX_STEPS.lock().await;
  *guard = cfg.max_agent_steps;
}

#[tauri::command]
#[specta::specta]
pub async fn game_sessions(app: AppHandle) -> Result<Vec<SessionV1>, AppError> {
  Ok(SessionV1::list(&app).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn game_page(app: AppHandle, session_id: String, page: u32) -> Result<PageV1, AppError> {
  let engine = GameEngine::from_app_and_session_id(&app, &session_id).await?;
  Ok(engine.page(page as usize).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn game_prompt(app: AppHandle, session_id: String, mode: GamePromptMode, prompt: String) -> Result<GamePromptResult, AppError> {
  let stream_id = Uuid::new_v4().to_string();
  let prompt = prompt.trim().to_string();
  let res = GamePromptResult { stream_id: stream_id.clone() };

  tokio::spawn(async move {
    if let Err(e) = agent_loop(&app, &session_id, &stream_id, mode, prompt).await {
      warn!("Error in agent loop: {}", e);
      let ev = GamePromptEvent::Error(GamePromptError::Internal(e));
      app.emit(&format!("gamePrompt[{}]", stream_id), ev).ok();
    }
  });

  Ok(res)
}

// NOTE: Uses the same events as game_prompt as it is really just a fork + prompt!
#[tauri::command]
#[specta::specta]
pub async fn game_fork(app: AppHandle, session_id: String, page_index: u32, prompt: String) -> Result<GamePromptResult, AppError> {
  let stream_id = Uuid::new_v4().to_string();
  let prompt = prompt.trim().to_string();
  let res = GamePromptResult { stream_id: stream_id.clone() };

  tokio::spawn(async move {
    if let Err(e) = fork_internal(&app, &session_id, &stream_id, page_index as usize, prompt).await {
      warn!("Error in agent loop: {}", e);
      let ev = GamePromptEvent::Error(GamePromptError::Internal(e));
      app.emit(&format!("gamePrompt[{}]", stream_id), ev).ok();
    }
  });

  Ok(res)
}

async fn fork_internal(app: &AppHandle, session_id: &String, stream_id: &String, page_index: usize, prompt: String) -> Result<(), AppError> {
  GameEngine::from_app_and_session_id(app, session_id).await?.fork(page_index).await?;
  agent_loop(app, session_id, stream_id, GamePromptMode::Write, prompt).await?;
  Ok(())
}

async fn agent_loop(app: &AppHandle, session_id: &String, stream_id: &String, mode: GamePromptMode, prompt: String) -> Result<AgentLoopResult, AppError> {
  let engine = GameEngine::from_app_and_session_id(app, session_id).await?;
  let history = engine.history(PageBatch::MAX_PAGES_PER_BATCH)?;
  let mut msgs: Vec<OpenAiChatMessage> = PageV1::to_openai_messages(&history);
  let mut step_idx = 0u8;
  let max_steps = get_max_steps().await;
  let client = reqwester().await;
  let config = Models::get(ModelUsage::Storyteller).await?;

  match mode {
    GamePromptMode::Write => {
      if prompt.is_empty() {
        msgs.push(OpenAiChatMessage::system(PROMPT_WRITE_MORE.to_string()));
      } else {
        msgs.push(OpenAiChatMessage::user(prompt));
      }
    }
    GamePromptMode::Instruct => {
      if prompt.is_empty() {
        return Err(AppError::no_input());
      }
      msgs.push(OpenAiChatMessage::system(PROMPT_INSTRUCT.to_string()));
      msgs.push(OpenAiChatMessage::user(prompt));
    }
    GamePromptMode::Steer => {
      if prompt.is_empty() {
        return Err(AppError::no_input())
      }
      msgs.push(OpenAiChatMessage::system(PROMPT_STEER.to_string()));
      msgs.push(OpenAiChatMessage::user(prompt));
    }
  }

  let mut done = false;
  let mut result = AgentLoopResult::default();
  while !done && step_idx < max_steps {
    let step_result = step(
      app,
      stream_id,
      &client,
      &config,
      &msgs,
      step_idx,
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
        for toolcall in toolcalls.iter() {
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
    let ev = GamePromptEvent::Done {
      finish_reason: result.finish_reason.clone(),
      usage: result.usage.clone(),
    };
    app.emit(&format!("gamePrompt[{}]", stream_id), ev).ok();
    Ok(result)
  }
}

async fn step(
  app: &AppHandle,
  stream_id: &String,
  client: &Client,
  config: &ModelConfig,
  msgs: &Vec<OpenAiChatMessage>,
  step_idx: u8,
) -> Result<StepResult, AppError> {
  let tools: Vec<OpenAiTool> = vec![];

  // TODO: truncate msgs & inject historical context
  let response = request_prompt(&client, &config, msgs.clone(), Some(tools)).await?;

  if !response.status().is_success() {
    let status = response.status().as_u16();
    let body = response.text().await.ok();
    let ev = GamePromptEvent::Error(GamePromptError::Request { status, body: body.clone() });
    app.emit(&format!("gamePrompt[{}]", stream_id), ev)?;
    return Err(AppError::llm(format!("LLM request failed with status {}, message: {}", status, body.unwrap_or("None".to_string()))));
  }

  let mut text: String = String::new();
  let mut toolcalls: HashMap<u32, ToolCall> = HashMap::new();

  let done = process_stream(response.bytes_stream(), &mut |event| match event {
    StreamEvent::Reasoning(delta) => {
      // We're not actually interested in the reasoning at all, beyond debugging in FE
      if !delta.is_empty() {
        let ev = GamePromptEvent::Reasoning { step: step_idx, delta };
        app.emit(&format!("gamePrompt[{}]", stream_id), ev).ok();
      }
    }
    StreamEvent::Text(delta) => {
      // Collect streaming text & pipe to FE
      if !delta.is_empty() {
        text += &delta;
        let ev = GamePromptEvent::Text { step: step_idx, delta };
        app.emit(&format!("gamePrompt[{}]", stream_id), ev).ok();
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

    // Event is emitted only for visualization & debugging purposes
    let ev = GamePromptEvent::ToolCalls {
      step: step_idx,
      text: text.clone(),
      calls: toolcalls.clone(),
    };
    app.emit(&format!("gamePrompt[{}]", stream_id), ev).ok();

    Ok(StepResult::ToolCalls {
      text,
      toolcalls,
    })
  } else {
    Ok(StepResult::Final(text, done))
  }
}
