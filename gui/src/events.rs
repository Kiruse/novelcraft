//! Shared resource for the account event stream (`GET /events`): a single
//! background supervisor owns the SSE connection for the whole app and
//! dispatches parsed events into global signals, so any number of
//! consumers (the title bar's `QuillsBox` today, future features) can
//! subscribe without holding their own stream.
//!
//! Lifecycle: the supervisor waits for a sign-in, seeds the balance from
//! `GET /quills`, then consumes the stream. Delivery is best-effort — on
//! a dropped or errored stream it reconnects after a pause, re-seeding
//! the balance first so missed events self-heal. A sign-out or account
//! switch drops the stale connection and clears the balance.

use std::time::Duration;
use std::sync::OnceLock;

use dioxus::prelude::*;
use log::warn;
use tokio::select;

use crate::api::{AccountEvent, client};
use crate::auth::AUTH;
use crate::time;

/// How often the supervisor re-checks the signed-in user — while waiting
/// for a sign-in, and during a live stream to notice sign-outs and
/// account switches.
const AUTH_POLL: Duration = Duration::from_millis(500);

/// Pause before reconnecting after a dropped or failed stream.
const RECONNECT_DELAY: Duration = Duration::from_secs(3);

/// The signed-in user's quill balance, as last reported by the server —
/// seeded from `GET /quills` on sign-in and refreshed by `quills` events.
/// Consumed by the title bar's [`crate::comps::QuillsBox`].
pub static QUILLS: GlobalSignal<Option<f64>> = GlobalSignal::new(|| None);

/// Starts the account-event supervisor (idempotent — one task for the
/// app's lifetime). Called from the boot gate, next to `auth::restore`.
pub fn spawn_supervisor() {
  static STARTED: OnceLock<()> = OnceLock::new();
  if STARTED.set(()).is_ok() {
    spawn(run());
  }
}

/// The supervisor loop: connect the stream for the signed-in user and
/// dispatch its events until it drops or the user changes.
async fn run() {
  loop {
    // Wait for a signed-in user.
    let user_id = loop {
      if let Some(user) = AUTH.read().user.clone() {
        break user.id;
      }
      time::sleep(AUTH_POLL).await;
    };

    // Seed the balance before the stream opens (events only carry
    // changes; the seed establishes the baseline).
    match client().quills_balance().await {
      Ok(balance) => *QUILLS.write() = Some(balance.quills),
      Err(err) => warn!("Failed to fetch the quill balance: {err}"),
    }
    let mut stream = match client().events_stream().await {
      Ok(stream) => stream,
      Err(err) => {
        warn!("Failed to open the account event stream: {err}");
        time::sleep(RECONNECT_DELAY).await;
        continue;
      }
    };

    // Consume events until the stream ends, errors, or the user changes.
    loop {
      if signed_in_user_id().as_deref() != Some(user_id.as_str()) {
        break;
      }
      select! {
        event = stream.next_event() => match event {
          Ok(Some(AccountEvent::Quills { quills })) => *QUILLS.write() = Some(quills),
          Ok(Some(AccountEvent::Unknown { event, .. })) => {
            warn!("Unhandled account event: {event}");
          }
          // The server closed the stream — reconnect (re-seeding the
          // balance so a missed event self-heals).
          Ok(None) => break,
          Err(err) => {
            warn!("Account event stream error: {err}");
            break;
          }
        },
        _ = time::sleep(AUTH_POLL) => {} // re-check the signed-in user
      }
    }

    if signed_in_user_id().as_deref() != Some(user_id.as_str()) {
      // The balance belongs to the previous user — clear it.
      *QUILLS.write() = None;
    } else {
      time::sleep(RECONNECT_DELAY).await;
    }
  }
}

/// The id of the signed-in user, if any.
fn signed_in_user_id() -> Option<String> {
  AUTH.read().user.as_ref().map(|user| user.id.clone())
}
