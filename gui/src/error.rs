use novelcraft_engine::error::EngineError;

#[derive(Debug, thiserror::Error)]
pub enum GuiError {
  #[error("Engine error: {0}")]
  Engine(#[from] EngineError),

  #[error("IO error: {0}")]
  Io(String),

  #[error("API error: {0}")]
  Api(String),
}

impl GuiError {
  #[inline(always)]
  pub fn io(msg: impl Into<String>) -> Self {
    Self::Io(msg.into())
  }

  #[inline(always)]
  pub fn api(msg: impl Into<String>) -> Self {
    Self::Api(msg.into())
  }
}
