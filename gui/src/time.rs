//! Platform-appropriate time helpers.
//!
//! `tokio::time` needs a runtime & timer driver — neither exists on wasm,
//! where it panics ("time not implemented on this platform"). The web uses
//! JS timers via gloo instead.

use std::time::Duration;

/// Sleep for the given duration.
pub(crate) async fn sleep(duration: Duration) {
  #[cfg(not(target_arch = "wasm32"))]
  tokio::time::sleep(duration).await;
  #[cfg(target_arch = "wasm32")]
  gloo_timers::future::TimeoutFuture::new(duration.as_millis() as u32).await;
}
