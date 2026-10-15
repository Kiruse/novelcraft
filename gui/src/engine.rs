use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, OnceLock};

use dioxus::prelude::*;
use log::error;
use novelcraft_engine::config::NovelCraftConfig;
use novelcraft_engine::error::EngineError;
use novelcraft_engine::game::engine::NovelCraftEngine;
use novelcraft_engine::game::session::SessionV1;
use novelcraft_engine::AgentMessageChunk;
use tokio::sync::Mutex;

use crate::state::{Profiles, Query, CONFIG, PAGE, PAGE_INDEX, PROFILES, SESSION, SESSIONS, STREAM};
use crate::toast::{push_toast, Toast};

static ENGINE: OnceLock<Arc<Mutex<NovelCraftEngine>>> = OnceLock::new();
static SESSIONS_GEN: AtomicUsize = AtomicUsize::new(0);
static PAGE_GEN: AtomicUsize = AtomicUsize::new(0);

/// Construct the process-wide engine. Must be called once before launch.
pub fn init_engine(config: NovelCraftConfig) {
  let _ = ENGINE.set(Arc::new(Mutex::new(NovelCraftEngine::new(config))));
}

#[inline]
fn engine() -> Arc<Mutex<NovelCraftEngine>> {
  ENGINE.get().expect("engine not initialized").clone()
}

// ── Queries ──────────────────────────────────────────────────────────────────

pub fn refresh_sessions() {
  let round = SESSIONS_GEN.fetch_add(1, Ordering::Relaxed) + 1;
  spawn(async move {
    let res = NovelCraftEngine::list_sessions()
      .await
      .map_err(|e| e.to_string());
    if SESSIONS_GEN.load(Ordering::Relaxed) == round {
      SESSIONS.write().publish(res);
    }
  });
}

#[allow(dead_code)]
pub async fn refresh_config_async() {
  let value = engine().lock().await.config().clone();
  CONFIG.write().publish(Ok(value));
}

#[allow(dead_code)]
pub fn refresh_config() {
  spawn(refresh_config_async());
}

pub async fn refresh_profiles_async() {
  let value = Profiles::from(engine().lock().await.config());
  PROFILES.write().publish(Ok(value));
}

#[allow(dead_code)]
pub fn refresh_profiles() {
  spawn(refresh_profiles_async());
}

pub async fn refresh_session_async() {
  let res = engine()
    .lock()
    .await
    .session()
    .cloned()
    .map_err(|e| e.to_string());
  SESSION.write().publish(res);
}

#[allow(dead_code)]
pub fn refresh_session() {
  spawn(refresh_session_async());
}

#[inline]
pub fn session_page_count() -> Option<usize> {
  SESSION.read().value().map(|session| session.page_count)
}

// ── Page navigation ──────────────────────────────────────────────────────────

/// Load the page at `index` into the global page query. Supersedes any
/// in-flight load — only the newest invocation publishes its result.
pub fn load_page(index: usize) {
  *PAGE_INDEX.write() = index;
  let round = PAGE_GEN.fetch_add(1, Ordering::Relaxed) + 1;
  spawn(async move {
    let res = engine()
      .lock()
      .await
      .page(index)
      .await
      .ok_or_else(|| EngineError::state(format!("page {index} not found")))
      .map_err(|e| e.to_string());
    if PAGE_GEN.load(Ordering::Relaxed) == round {
      PAGE.write().publish(res);
    }
  });
}

/// Load the page at `index`, clamped to the session's page count.
pub fn goto(index: usize) {
  let Some(count) = session_page_count() else { return };
  if count == 0 {
    return;
  }
  load_page(index.min(count - 1));
}

/// Load the next/previous page, clamped to the session's bounds.
pub fn navigate(delta: isize) {
  if STREAM.read().is_some() {
    return;
  }
  let Some(count) = session_page_count() else { return };
  if count == 0 {
    return;
  }
  let current = *PAGE_INDEX.read() as isize;
  let target = (current + delta).clamp(0, count as isize - 1) as usize;
  load_page(target);
}

pub fn jump_to_end() {
  let Some(count) = session_page_count() else { return };
  if count > 0 {
    goto(count - 1);
  }
}

// ── Commands ─────────────────────────────────────────────────────────────────

/// Create a new session, make it the engine's active session and publish
/// it to the session query. Toasts on failure.
pub async fn create_session(title: String, exposition: String) -> Result<SessionV1, EngineError> {
  let engine = engine();
  let mut eng = engine.lock().await;
  match eng.create_session(title, exposition).await {
    Ok(session) => {
      eng.set_session(Some(session.clone()));
      *SESSION.write() = Query::Recent(session.clone());
      Ok(session)
    }
    Err(err) => {
      error!("Error creating session: {err}");
      eng.set_session(None);
      *SESSION.write() = Query::Error(err.to_string());
      push_toast(Toast::error("Session creation failed"));
      Err(err)
    }
  }
}

/// Load the session with the given ID as the engine's active session and
/// publish it to the session query, opening its last page. Toasts on
/// failure.
pub async fn play_session(id: String) {
  // Discard the previous session's page & any in-flight load — a newly
  // entered session starts at page 0.
  *STREAM.write() = None;
  *PAGE_INDEX.write() = 0;
  *PAGE.write() = Query::Empty;
  *SESSION.write() = Query::Empty;

  let engine = engine();
  let res: Result<SessionV1, EngineError> = async {
    let mut eng = engine.lock().await;
    let session = SessionV1::load(id, &eng.config().profiles).await?;
    eng.set_session(Some(session.clone()));
    Ok(session)
  }
  .await;

  match res {
    Ok(session) => {
      *SESSION.write() = Query::Recent(session.clone());
      if session.page_count > 0 {
        load_page(session.page_count - 1);
      }
    }
    Err(err) => {
      error!("Error loading session: {err}");
      engine.lock().await.set_session(None);
      *SESSION.write() = Query::Error(err.to_string());
      push_toast(Toast::error("Failed to load session"));
    }
  }
}

/// Switch the active profile, persist it, and refresh both the profiles
/// and session queries (the active session runs with the new profile).
pub async fn switch_profile(id: String) {
  let engine = engine();
  let res: Result<(), EngineError> = async {
    let mut eng = engine.lock().await;
    eng.set_active_profile(Some(id));
    eng.save().await?;
    Ok(())
  }
  .await;

  if let Err(err) = res {
    error!("Failed to persist active profile: {err}");
    push_toast(Toast::error("Failed to save active profile"));
  }

  refresh_profiles_async().await;
  refresh_session_async().await;
}

/// Persist the config to disk, sync it with the engine, and publish it to
/// the config & profiles queries.
pub async fn save_config(config: NovelCraftConfig) -> Result<(), EngineError> {
  config.save().await?;
  engine().lock().await.set_config(config.clone());
  *CONFIG.write() = Query::Recent(config.clone());
  *PROFILES.write() = Query::Recent(Profiles::from(&config));
  Ok(())
}

/// Fork at `from_page` (if any) and submit `content` as a user prompt to
/// the active session, streaming agent chunks into the global stream
/// state. On completion the stream is cleared, the session query refreshed
/// and the current page reloaded — a fork may have truncated pages, and
/// the current page gained the fresh response either way.
pub fn prompt(content: String, from_page: Option<usize>) {
  *STREAM.write() = Some(crate::state::StreamState {
    prompt: content.clone(),
    ..Default::default()
  });

  let page_index = *PAGE_INDEX.read();
  spawn(async move {
    let engine = engine();
    let (tx, mut rx) = tokio::sync::mpsc::channel::<AgentMessageChunk>(100);

    // Drain chunks as they arrive; the sender is dropped when the worker
    // finishes, closing this loop.
    spawn(async move {
      while let Some(chunk) = rx.recv().await {
        if let Some(stream) = STREAM.write().as_mut() {
          stream.absorb(chunk);
        }
      }
    });

    let res: Result<(), EngineError> = async {
      let mut eng = engine.lock().await;
      if let Some(page) = from_page {
        eng.fork(page).await?;
      }
      eng.prompt(content, tx).await
    }
    .await;

    *STREAM.write() = None;
    if let Err(err) = res {
      error!("Prompt failed: {err}");
      push_toast(Toast::error(format!("Prompt failed: {err}")));
    }

    refresh_session_async().await;
    load_page(page_index);
  });
}
