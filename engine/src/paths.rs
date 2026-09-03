use std::path::PathBuf;

use crate::error::EngineError;

pub fn data_dir() -> Result<PathBuf, EngineError> {
  dirs::data_dir()
    .ok_or_else(|| EngineError::internal("Could not resolve data directory"))
    .map(|p| p.join("NovelCraft"))
}

pub fn config_dir() -> Result<PathBuf, EngineError> {
  dirs::config_dir()
    .ok_or_else(|| EngineError::internal("Could not resolve config directory"))
    .map(|p| p.join("NovelCraft"))
}

pub fn sessions_dir() -> Result<PathBuf, EngineError> {
  Ok(data_dir()?.join("sessions"))
}
