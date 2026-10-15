use novelcraft_engine::error::EngineError;

#[derive(Debug, thiserror::Error)]
pub enum GuiError {
  #[error(transparent)]
  Engine(#[from] EngineError),
  #[error("{0}")]
  Api(String),
}
