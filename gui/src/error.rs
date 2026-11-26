use novelcraft_engine::error::EngineError;

#[derive(Debug, thiserror::Error)]
pub enum GuiError {
  #[error(transparent)]
  Engine(#[from] EngineError),

  /// An error response from the social API: the HTTP status and the
  /// server's safe public message.
  #[error("{status}: {message}")]
  ApiStatus {
    status: u16,
    message: String,
  },

  #[error("{0}")]
  Api(String),

  #[error("{0}")]
  Config(String),
}
