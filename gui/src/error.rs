use novelcraft_engine::error::EngineError;

#[derive(Debug, thiserror::Error)]
pub enum GuiError {
  #[error("Engine error: {0}")]
  Engine(#[from] EngineError),

  #[error("API error: {0}")]
  Api(String),
}
