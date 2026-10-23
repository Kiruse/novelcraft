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
      AgentMessageChunk::Content { text } => {
        // Prose is flowing again — stop showing the tool-activity spinner
        // so the incoming text actually streams in.
        self.activity = false;
        self.text.push_str(&text);
      }
      AgentMessageChunk::ToolCallStart { .. } | AgentMessageChunk::ToolCallArgs { .. } => {
        self.activity = true;
      }
      AgentMessageChunk::Reasoning { .. } | AgentMessageChunk::Done { .. } => {}
    }
  }
}

/// Reasoning of the latest generation, captured from `Reasoning` chunks by
/// the stream drain task and kept after the stream closes for the sidebar's
/// Reasoning section.
pub static REASONING: GlobalSignal<String> = GlobalSignal::new(String::new);

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn activity_toggles_with_chunks() {
    let mut stream = StreamState::default();

    stream.absorb(AgentMessageChunk::ToolCallStart {
      index: 0,
      id: "1".to_string(),
      name: "story::note_events".to_string(),
    });
    assert!(stream.activity);

    stream.absorb(AgentMessageChunk::Content { text: "hello ".to_string() });
    assert!(!stream.activity);
    assert_eq!(stream.text, "hello ");

    stream.absorb(AgentMessageChunk::ToolCallArgs { index: 0, args: "{}".to_string() });
    assert!(stream.activity);

    stream.absorb(AgentMessageChunk::Content { text: "world".to_string() });
    assert!(!stream.activity);
    assert_eq!(stream.text, "hello world");
  }
}

/// The config the engine booted with. Set during the `App` boot gate —
/// the config & profiles queries lazily seed themselves from it on first
/// read inside the Dioxus runtime.
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
/// View position within the gameplay screen: `-1` is the premise page
/// (virtual page 0, before the first turn), `0..` are turn indices.
pub const PREMISE_PAGE: isize = -1;
pub static TURN_INDEX: GlobalSignal<isize> = GlobalSignal::new(|| PREMISE_PAGE);
pub static TURN: GlobalSignal<Query<TurnV2>> = GlobalSignal::new(|| Query::Empty);
pub static STREAM: GlobalSignal<Option<StreamState>> = GlobalSignal::new(|| None);
