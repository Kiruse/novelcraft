use std::path::PathBuf;

use crate::commands::paths as cmd_paths;
use crate::error::AppError;

pub async fn datapath(path: String) -> Result<String, AppError> {
  let basepath = cmd_paths::data_dir()?;
  let basepath = std::fs::canonicalize(&basepath).map_err(|e| AppError::io(e.to_string()))?;

  let path = basepath.join(PathBuf::from(path));
  if !path.starts_with(basepath) {
    return Err(AppError::Path("Path out of bounds".to_string()));
  }

  Ok(path.to_string_lossy().to_string())
}
