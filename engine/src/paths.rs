use std::path::PathBuf;

use crate::error::EngineError;

/// Root of all persisted app data (sessions, snapshots). On desktop this is
/// the platform data directory; on the web it is a logical namespace rooted
/// at the OPFS root (interpreted by `util::fs::opfs`).
pub fn data_dir() -> Result<PathBuf, EngineError> {
  #[cfg(not(target_arch = "wasm32"))]
  let dir = dirs::data_dir()
    .ok_or_else(|| EngineError::internal("Could not resolve data directory"))
    .map(|p| p.join("NovelCraft"));
  #[cfg(target_arch = "wasm32")]
  let dir = Ok(PathBuf::from("data/NovelCraft"));
  dir
}

/// Root of persisted app config. See [`data_dir`].
pub fn config_dir() -> Result<PathBuf, EngineError> {
  #[cfg(not(target_arch = "wasm32"))]
  let dir = dirs::config_dir()
    .ok_or_else(|| EngineError::internal("Could not resolve config directory"))
    .map(|p| p.join("NovelCraft"));
  #[cfg(target_arch = "wasm32")]
  let dir = Ok(PathBuf::from("config/NovelCraft"));
  dir
}

pub fn sessions_dir() -> Result<PathBuf, EngineError> {
  Ok(data_dir()?.join("sessions"))
}
