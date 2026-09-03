use std::path::Path;

use crate::error::EngineError;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serializer};
use serde::{Serialize, de::DeserializeOwned};
use tokio::sync::mpsc::Sender;

pub mod prompting;

pub(crate) async fn ensure_dir(path: &Path) -> Result<(), EngineError> {
  tokio::fs::create_dir_all(path)
    .await
    .map_err(|e| EngineError::io(format!("Failed to create directory: {}", e)))
}

pub(crate) async fn deserialize<T: DeserializeOwned>(path: &Path) -> Result<T, EngineError> {
  let raw = tokio::fs::read_to_string(path)
    .await
    .map_err(|e| EngineError::io(format!("Read error: {}", e)))?;
  serde_json::from_str(&raw).map_err(|e| EngineError::parse(format!("Parse error: {}", e)))
}

pub(crate) async fn serialize(path: &Path, obj: &impl Serialize) -> Result<(), EngineError> {
  if let Some(parent) = path.parent() {
    ensure_dir(parent).await?;
  }
  let content =
    serde_json::to_string_pretty(obj).map_err(|e| EngineError::parse(format!("Serialize error: {}", e)))?;
  tokio::fs::write(path, content)
    .await
    .map_err(|e| EngineError::io(format!("Write error: {}", e)))
}

pub(crate) fn serialize_timestamp<S>(value: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error>
where S: Serializer
{
  serializer.serialize_str(&value.to_rfc3339())
}

pub(crate) fn deserialize_timestamp<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
where D: Deserializer<'de>
{
  let s = String::deserialize(deserializer)?;
  Ok(DateTime::parse_from_rfc3339(&s)
    .map_err(|e| serde::de::Error::custom(e.to_string()))?
    .to_utc())
}

pub(crate) fn discard_channel<T: Send + 'static>() -> Sender<T> {
  let (tx, mut rx) = tokio::sync::mpsc::channel(100);
  tokio::spawn(async move {
    while rx.recv().await.is_some() {}
  });
  tx
}
