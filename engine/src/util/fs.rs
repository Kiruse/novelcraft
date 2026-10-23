//! Filesystem abstraction — cfg-split storage seam.
//!
//! Desktop speaks to real files via `tokio::fs`; the wasm32 target speaks
//! to the browser's OPFS via web-sys (see [`opfs`]). Both impls share the
//! same signatures, so engine code above this layer is target-agnostic.
//! Web paths are logical namespaces resolved from the OPFS root (see
//! `crate::paths`).

use std::path::Path;

use crate::error::EngineError;

use serde::{Serialize, de::DeserializeOwned};

#[cfg(target_arch = "wasm32")]
pub(crate) mod opfs;

/// Create a directory and all of its parents.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) async fn ensure_dir(path: &Path) -> Result<(), EngineError> {
  tokio::fs::create_dir_all(path)
    .await
    .map_err(|e| EngineError::io(format!("Failed to create directory: {}", e)))
}

/// Create a directory and all of its parents.
#[cfg(target_arch = "wasm32")]
pub(crate) async fn ensure_dir(path: &Path) -> Result<(), EngineError> {
  opfs::ensure_dir(path).await
}

/// Remove a file; a missing file is a no-op.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) async fn remove_file(path: &Path) -> Result<(), EngineError> {
  match tokio::fs::remove_file(path).await {
    Ok(()) => Ok(()),
    Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
    Err(e) => Err(EngineError::io(format!("Remove error: {}", e))),
  }
}

/// Remove a file; a missing file is a no-op.
#[cfg(target_arch = "wasm32")]
pub(crate) async fn remove_file(path: &Path) -> Result<(), EngineError> {
  opfs::remove_file(path).await
}

/// List a directory's entries as `(name, is_directory)`. A missing
/// directory yields an empty list.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) async fn list_dir(path: &Path) -> Result<Vec<(String, bool)>, EngineError> {
  let mut dir = match tokio::fs::read_dir(path).await {
    Ok(dir) => dir,
    Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
    Err(e) => return Err(EngineError::io(format!("Read dir error: {}", e))),
  };
  let mut entries = Vec::new();
  while let Some(entry) = dir
    .next_entry()
    .await
    .map_err(|e| EngineError::io(format!("Read dir error: {}", e)))?
  {
    let is_dir = entry
      .file_type()
      .await
      .map_err(|e| EngineError::io(format!("Read dir error: {}", e)))?
      .is_dir();
    entries.push((entry.file_name().to_string_lossy().to_string(), is_dir));
  }
  Ok(entries)
}

/// List a directory's entries as `(name, is_directory)`. A missing
/// directory yields an empty list.
#[cfg(target_arch = "wasm32")]
pub(crate) async fn list_dir(path: &Path) -> Result<Vec<(String, bool)>, EngineError> {
  opfs::list_dir(path).await
}

/// Read & parse a JSON file.
pub(crate) async fn deserialize<T: DeserializeOwned>(path: &Path) -> Result<T, EngineError> {
  let raw = read(path).await?;
  serde_json::from_str(&raw).map_err(|e| EngineError::parse(format!("Parse error: {}", e)))
}

/// Write a JSON file (pretty-printed), creating parent directories.
pub(crate) async fn serialize(path: &Path, obj: &impl Serialize) -> Result<(), EngineError> {
  if let Some(parent) = path.parent() {
    ensure_dir(parent).await?;
  }
  let content =
    serde_json::to_string_pretty(obj).map_err(|e| EngineError::parse(format!("Serialize error: {}", e)))?;
  write(path, &content).await
}

/// Read a file's whole text content.
#[cfg(not(target_arch = "wasm32"))]
async fn read(path: &Path) -> Result<String, EngineError> {
  tokio::fs::read_to_string(path)
    .await
    .map_err(|e| EngineError::io(format!("Read error: {}", e)))
}

/// Read a file's whole text content.
#[cfg(target_arch = "wasm32")]
async fn read(path: &Path) -> Result<String, EngineError> {
  opfs::read(path).await
}

/// Write a file's whole text content.
#[cfg(not(target_arch = "wasm32"))]
async fn write(path: &Path, content: &str) -> Result<(), EngineError> {
  tokio::fs::write(path, content)
    .await
    .map_err(|e| EngineError::io(format!("Write error: {}", e)))
}

/// Write a file's whole text content.
#[cfg(target_arch = "wasm32")]
async fn write(path: &Path, content: &str) -> Result<(), EngineError> {
  opfs::write(path, content).await
}
