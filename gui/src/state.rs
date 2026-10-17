use std::sync::OnceLock;

use dioxus::prelude::*;
use novelcraft_engine::config::NovelCraftConfig;
use novelcraft_engine::game::turns::TurnV2;
use novelcraft_engine::game::profile::ProfileV1;
use novelcraft_engine::game::session::SessionV2;
use novelcraft_engine::AgentMessageChunk;

/// Result of an engine query, mirroring the semantics of the previous
/// gpui `EngineQuery`: a fresh value, an error without value, or a stale
/// value retained alongside a refresh error.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum Query<T> {
  #[default]
  Empty,
  Error(String),
  Stale(T, String),
  Recent(T),
}

impl<T: Clone> Query<T> {
  /// Publish the outcome of a refresh, retaining any previous value as
  /// stale on error.
  pub fn publish(&mut self, result: Result<T, String>) {
    *self = match result {
      Ok(value) => Query::Recent(value),
      Err(error) => match std::mem::replace(self, Query::Empty) {
        Query::Recent(value) | Query::Stale(value, _) => Query::Stale(value, error),
        _ => Query::Error(error),
      },
    };
  }

  pub fn value(&self) -> Option<&T> {
    match self {
      Query::Recent(value) | Query::Stale(value, _) => Some(value),
      _ => None,
    }
  }

  pub fn error(&self) -> Option<&String> {
    match self {
      Query::Error(error) | Query::Stale(_, error) => Some(error),
      _ => None,
    }
  }

  pub fn is_stale(&self) -> bool {
    matches!(self, Query::Stale(..))
  }
}

/// Snapshot of the config's user profiles.
#[derive(Debug, Clone, Default)]
pub struct Profiles {
  pub profiles: Vec<ProfileV1>,
  pub active_profile: Option<String>,
}

impl Profiles {
  #[allow(dead_code)]
  pub fn active(&self) -> Option<&ProfileV1> {
    let active = self.active_profile.as_ref()?;
    self.profiles.iter().find(|p| p.id == *active)
  }
}

impl From<&NovelCraftConfig> for Profiles {
  fn from(config: &NovelCraftConfig) -> Self {
    Self {
      profiles: config.profiles.clone(),
      active_profile: config.active_profile.clone(),
    }
  }
}

/// In-flight generation state, rendered in place of the page content.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct StreamState {
  pub prompt: String,
  pub text: String,
  pub activity: bool,
}

impl StreamState {
  pub fn absorb(&mut self, chunk: AgentMessageChunk) {
    match chunk {
      AgentMessageChunk::Content { text } => self.text.push_str(&text),
      AgentMessageChunk::ToolCallStart { .. } | AgentMessageChunk::ToolCallArgs { .. } => {
        self.activity = true;
      }
      AgentMessageChunk::Reasoning { .. } | AgentMessageChunk::Done { .. } => {}
    }
  }
}

/// The config the engine booted with. Set before launch — the config &
/// profiles queries lazily seed themselves from it on first read inside
/// the Dioxus runtime.
pub static BOOT_CONFIG: OnceLock<NovelCraftConfig> = OnceLock::new();

pub static CONFIG: GlobalSignal<Query<NovelCraftConfig>> = GlobalSignal::new(|| {
  BOOT_CONFIG
    .get()
    .map(|config| Query::Recent(config.clone()))
    .unwrap_or(Query::Empty)
});
pub static PROFILES: GlobalSignal<Query<Profiles>> = GlobalSignal::new(|| {
  BOOT_CONFIG
    .get()
    .map(|config| Query::Recent(Profiles::from(config)))
    .unwrap_or(Query::Empty)
});
pub static SESSIONS: GlobalSignal<Query<Vec<SessionV2>>> = GlobalSignal::new(|| Query::Empty);
pub static SESSION: GlobalSignal<Query<SessionV2>> = GlobalSignal::new(|| Query::Empty);
pub static TURN_INDEX: GlobalSignal<usize> = GlobalSignal::new(|| 0);
pub static TURN: GlobalSignal<Query<TurnV2>> = GlobalSignal::new(|| Query::Empty);
pub static STREAM: GlobalSignal<Option<StreamState>> = GlobalSignal::new(|| None);
