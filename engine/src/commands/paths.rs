use std::path::{Path, PathBuf};

use crate::error::AppError;

pub fn data_dir() -> Result<PathBuf, AppError> {
  dirs::data_dir()
    .ok_or_else(|| AppError::internal("Could not resolve data directory"))
    .map(|p| p.join("NovelCraft"))
}

pub fn config_dir() -> Result<PathBuf, AppError> {
  dirs::config_dir()
    .ok_or_else(|| AppError::internal("Could not resolve config directory"))
    .map(|p| p.join("NovelCraft"))
}

pub fn sessions_dir() -> Result<PathBuf, AppError> {
  Ok(data_dir()?.join("sessions"))
}

pub fn session_dir(id: &str) -> Result<PathBuf, AppError> {
  Ok(sessions_dir()?.join(id))
}

pub fn session_meta_path(id: &str) -> Result<PathBuf, AppError> {
  Ok(session_dir(id)?.join("meta.json"))
}

pub fn stories_dir() -> Result<PathBuf, AppError> {
  Ok(data_dir()?.join("stories"))
}

pub fn lore_dir() -> Result<PathBuf, AppError> {
  Ok(data_dir()?.join("lore"))
}

pub fn models_config_path() -> Result<PathBuf, AppError> {
  config_dir().map(|p| p.join("models.json"))
}

pub fn profiles_path() -> Result<PathBuf, AppError> {
  data_dir().map(|p| p.join("profiles.json"))
}

pub fn canonical_path(path: &Path) -> Result<PathBuf, AppError> {
  std::fs::canonicalize(path).map_err(|e| AppError::io(e.to_string()))
}
