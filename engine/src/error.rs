use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, thiserror::Error, Serialize, Deserialize, Type)]
#[serde(tag = "kind", content = "detail", rename_all = "snake_case")]
pub enum AppError {
  #[error("IO error: {0}")]
  Io(String),

  #[error("not found: {0}")]
  NotFound(String),

  #[error("validation error: {0}")]
  Validation(String),

  #[error("LLM error: {0}")]
  Llm(String),

  #[error("path error: {0}")]
  Path(String),

  #[error("parse error: {0}")]
  Parse(String),

  #[error("input error: {0}")]
  Input(String),

  #[error("invalid state: {0}")]
  State(String),

  #[error("{0}")]
  Internal(String),
}

impl AppError {
  pub fn not_found(msg: impl Into<String>) -> Self {
    Self::NotFound(msg.into())
  }

  pub fn validation(msg: impl Into<String>) -> Self {
    Self::Validation(msg.into())
  }

  pub fn io(msg: impl Into<String>) -> Self {
    Self::Io(msg.into())
  }

  pub fn llm(msg: impl Into<String>) -> Self {
    Self::Llm(msg.into())
  }

  pub fn parse(msg: impl Into<String>) -> Self {
    Self::Parse(msg.into())
  }

  pub fn input(msg: impl Into<String>) -> Self {
    Self::Input(msg.into())
  }

  pub fn no_input() -> Self {
    Self::input("no input provided")
  }

  pub fn state(msg: impl Into<String>) -> Self {
    Self::State(msg.into())
  }

  pub fn internal(msg: impl Into<String>) -> Self {
    Self::Internal(msg.into())
  }
}

impl From<std::io::Error> for AppError {
  fn from(e: std::io::Error) -> Self {
    Self::Io(e.to_string())
  }
}

impl From<tauri::Error> for AppError {
  fn from(e: tauri::Error) -> Self {
    Self::internal(e.to_string())
  }
}

impl From<serde_json::Error> for AppError {
  fn from(e: serde_json::Error) -> Self {
    Self::Parse(e.to_string())
  }
}
