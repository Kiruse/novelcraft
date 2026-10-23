use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, OnceLock};

use dioxus::prelude::*;
use log::error;
use novelcraft_engine::config::NovelCraftConfig;
use novelcraft_engine::error::EngineError;
use novelcraft_engine::game::engine::NovelCraftEngine;
use novelcraft_engine::game::profile::ProfileV1;
use novelcraft_engine::game::session::SessionV2;
use novelcraft_engine::AgentMessageChunk;
use tokio::sync::Mutex;

use crate::state::{Profiles, Query, CONFIG, PREMISE_PAGE, PROFILES, REASONING, SESSION, SESSIONS, STREAM, TURN, TURN_INDEX};
use crate::toast::{push_toast, Toast};

static ENGINE: OnceLock<Arc<Mutex<NovelCraftEngine>>> = OnceLock::new();
static SESSIONS_GEN: AtomicUsize = AtomicUsize::new(0);
static TURN_GEN: AtomicUsize = AtomicUsize::new(0);

/// Construct the process-wide engine. Must be called exactly once, before
/// any engine operation (done by the `App` boot gate).
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
pub fn session_turn_count() -> Option<usize> {
  SESSION.read().value().map(|session| session.turn_count())
}

// ── Turn navigation ──────────────────────────────────────────────────────────

/// Load the turn at `index` (relative to the retained history) into the
/// global turn query. Supersedes any in-flight load — only the newest
/// invocation publishes its result.
pub fn load_turn(index: usize) {
  *TURN_INDEX.write() = index as isize;
  let round = TURN_GEN.fetch_add(1, Ordering::Relaxed) + 1;
  spawn(async move {
    let res = engine()
      .lock()
      .await
      .turn(index)
      .ok_or_else(|| EngineError::state(format!("turn {index} not found")))
      .map_err(|e| e.to_string());
    if TURN_GEN.load(Ordering::Relaxed) == round {
      TURN.write().publish(res);
    }
  });
}

/// Show the premise page (virtual page 0, before the first turn).
pub fn load_premise() {
  *TURN_INDEX.write() = PREMISE_PAGE;
}

/// Load the turn at `index`, clamped to the retained history's bounds.
pub fn goto(index: usize) {
  let Some(count) = session_turn_count() else { return };
  if count == 0 {
    return;
  }
  load_turn(index.min(count - 1));
}

/// Load the next/previous page, clamped to the premise page and the
/// retained history's bounds.
pub fn navigate(delta: isize) {
  if STREAM.read().is_some() {
    return;
  }
  let Some(count) = session_turn_count() else { return };
  if count == 0 {
    return;
  }
  let current = *TURN_INDEX.read();
  let target = (current + delta).clamp(PREMISE_PAGE, count as isize - 1);
  if target == PREMISE_PAGE {
    load_premise();
  } else {
    load_turn(target as usize);
  }
}

pub fn jump_to_end() {
  let Some(count) = session_turn_count() else { return };
  if count > 0 {
    goto(count - 1);
  }
}

// ── Commands ─────────────────────────────────────────────────────────────────

/// Create a new session, make it the engine's active session and publish
/// it to the session query. Toasts on failure.
pub async fn create_session(title: String, exposition: String) -> Result<SessionV2, EngineError> {
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
/// publish it to the session query, opening its last retained turn. Toasts
/// on failure.
pub async fn play_session(id: String) {
  // Discard the previous session's turn & any in-flight load — a newly
  // entered session opens on its premise page (or last turn, below).
  *STREAM.write() = None;
  *REASONING.write() = String::new();
  load_premise();
  *TURN.write() = Query::Empty;
  *SESSION.write() = Query::Empty;

  let engine = engine();
  let res: Result<SessionV2, EngineError> = async {
    let mut eng = engine.lock().await;
    let session = SessionV2::load(id, &eng.config().profiles).await?;
    eng.set_session(Some(session.clone()));
    Ok(session)
  }
  .await;

  match res {
    Ok(session) => {
      *SESSION.write() = Query::Recent(session.clone());
      if session.turn_count() > 0 {
        load_turn(session.turn_count() - 1);
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

/// Delete the session with the given ID from disk. If it was the engine's
/// active session, the active session (and its queries) are reset. Toasts
/// on failure.
pub async fn delete_session(id: String) -> Result<(), EngineError> {
  let engine = engine();
  let res: Result<(), EngineError> = async {
    let mut eng = engine.lock().await;
    SessionV2::delete(&id).await?;
    if eng.session().is_ok_and(|session| session.id == id) {
      eng.set_session(None);
      *SESSION.write() = Query::Empty;
      *TURN.write() = Query::Empty;
      load_premise();
      *STREAM.write() = None;
    }
    Ok(())
  }
  .await;

  if let Err(ref err) = res {
    error!("Failed to delete session: {err}");
    push_toast(Toast::error("Failed to delete session"));
  } else {
    refresh_sessions();
  }
  res
}

/// Create a new, empty profile, make it the active one and persist the
/// config. Refreshes the profiles & session queries on success; toasts on
/// failure.
pub async fn create_profile_async() -> Result<ProfileV1, EngineError> {
  let res = engine().lock().await.create_profile().await;
  if let Err(ref err) = res {
    error!("Failed to create profile: {err}");
    push_toast(Toast::error("Failed to create profile"));
  }
  if res.is_ok() {
    refresh_profiles_async().await;
    refresh_session_async().await;
  }
  res
}

/// Persist edits to an existing profile. Refreshes the profiles (and, for
/// the active profile, session) queries on success; toasts on failure.
pub async fn update_profile_async(profile: ProfileV1) -> Result<(), EngineError> {
  let res = engine().lock().await.update_profile(profile).await;
  if let Err(ref err) = res {
    error!("Failed to save profile: {err}");
    push_toast(Toast::error("Failed to save profile"));
  } else {
    refresh_profiles_async().await;
    refresh_session_async().await;
  }
  res
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

/// Read the active session's raw state for a gameplay module, for sidebar
/// rendering.
pub async fn module_state_async(modid: String) -> Result<serde_json::Value, EngineError> {
  engine().lock().await.module_state(&modid).await
}

/// Apply a manual gamestate edit from the sidebar: `keypath` is a full
/// path (first segment = module ID) and `value` the string override — an
/// empty string removes the field. The edit applies to the live gamestate
/// and is recorded on the last turn so replay reproduces it. Toasts on
/// failure.
pub async fn edit_module_state_async(
  keypath: Vec<String>,
  value: String,
) -> Result<(), EngineError> {
  let res = engine().lock().await.edit_module_state(keypath, value).await;
  if let Err(ref err) = res {
    error!("Failed to save module state: {err}");
    push_toast(Toast::error("Failed to save module changes"));
  }
  res
}

/// Build the effective system prompt (persona, premise, module contexts,
/// player profile) for the active session, for display in the sidebar.
pub async fn effective_prompt_async() -> Result<String, EngineError> {
  engine().lock().await.effective_prompt().await
}

/// Persist an edit to the last turn's user prompt (inline editing, saved
/// on blur). Refreshes the session & current turn queries on success.
pub async fn set_last_prompt_async(content: String) {
  let res = engine().lock().await.set_last_prompt(content).await;
  match res {
    Ok(()) => {
      refresh_session_async().await;
      // Read the index into a local first — the read guard must be dropped
      // before load_turn writes the global signal again. Prompt editing
      // only happens on turn pages, never on the premise page.
      let turn_index = *TURN_INDEX.read();
      if turn_index >= 0 {
        load_turn(turn_index as usize);
      }
    }
    Err(err) => {
      error!("Failed to save prompt: {err}");
      push_toast(Toast::error(format!("Failed to save prompt: {err}")));
    }
  }
}

/// Regenerate the last turn's agent response: the engine forks back past
/// it (or drops the entire history for the very first turn) and re-runs
/// the same user prompt, streaming into the global stream state just like
/// a regular prompt.
pub fn regenerate() {
  if STREAM.read().is_some() {
    return;
  }
  let Some(count) = session_turn_count() else { return };
  if count == 0 {
    return;
  }
  *REASONING.write() = String::new();

  // Show the re-run prompt in the stream card while regenerating.
  let prompt_text = SESSION
    .read()
    .value()
    .and_then(|session| session.turns.last())
    .and_then(|turn| turn.prompt.clone())
    .unwrap_or_default();
  *STREAM.write() = Some(crate::state::StreamState {
    prompt: prompt_text,
    ..Default::default()
  });

  let turn_index = count - 1;
  spawn(async move {
    let engine = engine();
    let (tx, rx) = tokio::sync::mpsc::channel::<AgentMessageChunk>(100);

    // Drain chunks as they arrive; the sender is dropped when the worker
    // finishes, closing this loop.
    spawn(drain_chunks(rx));

    let res = engine.lock().await.regenerate(tx).await;

    *STREAM.write() = None;
    if let Err(err) = res {
      error!("Regenerate failed: {err}");
      push_toast(Toast::error(format!("Regenerate failed: {err}")));
    }

    refresh_session_async().await;
    load_turn(turn_index);
  });
}

/// Drain a generation's chunk stream: absorb everything into the global
/// stream state and mirror reasoning into the global reasoning buffer (it
/// outlives the stream for the sidebar's Reasoning section).
async fn drain_chunks(mut rx: tokio::sync::mpsc::Receiver<AgentMessageChunk>) {
  while let Some(chunk) = rx.recv().await {
    if let AgentMessageChunk::Reasoning { text } = &chunk {
      REASONING.write().push_str(text);
    }
    if let Some(stream) = STREAM.write().as_mut() {
      stream.absorb(chunk);
    }
  }
}

/// Fork at `from_turn` (if any) and submit `content` as a user prompt to
/// the active session, streaming agent chunks into the global stream
/// state. On completion the stream is cleared, the session query refreshed
/// and the current turn reloaded — a fork may have truncated turns, and
/// the current turn gained the fresh response either way.
pub fn prompt(content: String, from_turn: Option<usize>) {
  *REASONING.write() = String::new();
  *STREAM.write() = Some(crate::state::StreamState {
    prompt: content.clone(),
    ..Default::default()
  });

    let turn_index = *TURN_INDEX.read();
    spawn(async move {
      let engine = engine();
      let (tx, rx) = tokio::sync::mpsc::channel::<AgentMessageChunk>(100);

      // Drain chunks as they arrive; the sender is dropped when the worker
      // finishes, closing this loop.
      spawn(drain_chunks(rx));

      let res: Result<(), EngineError> = async {
        let mut eng = engine.lock().await;
        if let Some(turn) = from_turn {
          eng.fork(turn).await?;
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
      // The fresh response always sits one page after the prompt's origin
      // (both for plain pushes and for forks).
      load_turn(turn_index as usize + 1);
    });
}

/// Submit `content` as a fresh first prompt from the premise page: the
/// retained history is dropped (the gamestate is restored to the session
/// snapshot) and the DM starts the story anew. Streams like a regular
/// prompt.
pub fn start_over(content: String) {
  if STREAM.read().is_some() {
    return;
  }
  *REASONING.write() = String::new();
  *STREAM.write() = Some(crate::state::StreamState {
    prompt: content.clone(),
    ..Default::default()
  });

  spawn(async move {
    let engine = engine();
    let (tx, rx) = tokio::sync::mpsc::channel::<AgentMessageChunk>(100);

    // Drain chunks as they arrive; the sender is dropped when the worker
    // finishes, closing this loop.
    spawn(drain_chunks(rx));

    let res: Result<(), EngineError> = async {
      let mut eng = engine.lock().await;
      eng.reset_history().await?;
      eng.prompt(content, tx).await
    }
    .await;

    *STREAM.write() = None;
    if let Err(err) = res {
      error!("Start over failed: {err}");
      push_toast(Toast::error(format!("Failed to restart the story: {err}")));
    }

    refresh_session_async().await;
    load_turn(0);
  });
}
