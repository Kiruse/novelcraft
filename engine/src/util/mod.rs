use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serializer};
use tokio::sync::mpsc::Sender;

pub mod fs;
pub mod prompting;

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

/// A channel whose messages are silently drained in the background (e.g.
/// LLM calls whose chunks nobody consumes). The drainer runs on tokio
/// (native) or the JS microtask queue (web) — `tokio::spawn` would panic
/// on wasm, where no tokio runtime exists.
pub(crate) fn discard_channel<T: Send + 'static>() -> Sender<T> {
  let (tx, mut rx) = tokio::sync::mpsc::channel(100);
  #[cfg(not(target_arch = "wasm32"))]
  tokio::spawn(async move {
    while rx.recv().await.is_some() {}
  });
  #[cfg(target_arch = "wasm32")]
  wasm_bindgen_futures::spawn_local(async move {
    while rx.recv().await.is_some() {}
  });
  tx
}
