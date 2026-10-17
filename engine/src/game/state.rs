use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use serde::de::DeserializeOwned;
use serde::{Serialize};
use serde_json::{Map as JsonMap, Value as JsonValue, json};
use tokio::sync::RwLock;

use crate::error::EngineError;
use crate::game::module::GameplayModule;
use crate::game::profile::ProfileV1;
use crate::game::turns::TurnV2;

/// Gamestate snapshot: module ID => module state as a JSON object.
pub type Snapshot = JsonMap<String, JsonValue>;

/// GameState is effectively a handle to the shared game state.
/// Every module has its own [RwLock]ed state, and it is the
/// engine's responsibility to order module's tool calls by
/// dependencies on other modules.
///
/// The GameState itself can be safely cloned. However, the
/// module registry is not intended to be updated, meaning it
/// is not possible to add or remove modules without creating
/// a new handle.
#[derive(Debug, Clone, Default)]
pub struct GameState {
  storage: Arc<HashMap<String, ModuleState>>,
  pub profile: Option<Arc<ProfileV1>>,
}

impl GameState {
  /// Build a game state from the given snapshot. Module IDs missing from the
  /// snapshot are seeded with empty state so that modules can always write
  /// their keys.
  pub fn new(
    snapshot: Snapshot,
    module_ids: impl IntoIterator<Item = String>,
    profile: Option<Arc<ProfileV1>>,
  ) -> Result<Self, EngineError> {
    let mut snapshot = snapshot;
    for id in module_ids {
      snapshot.entry(id).or_insert_with(|| json!({}));
    }
    Ok(Self {
      storage: Self::build_storage(snapshot)?,
      profile,
    })
  }

  #[inline(always)]
  pub fn set_profile(&mut self, profile: Option<Arc<ProfileV1>>) {
    self.profile = profile;
  }

  /// Get a readonly [ModuleState] of the ID'ed module.
  pub fn get(&self, module: &String) -> Option<&ModuleState> {
    self.storage.get(module)
  }

  /// Mark named module dirty. Errors if no such module exists.
  pub fn mark_dirty(&self, module: &String) -> Result<(), EngineError> {
    let Some(module) = self.storage.get(module) else {
      return Err(EngineError::module_not_found(module));
    };
    module.dirty.store(true, Ordering::Release);
    Ok(())
  }

  /// Mark named module clean & return whether it *was* dirty. Errors if no
  /// such module exists.
  pub(crate) fn mark_clean(&self, module: &String) -> Result<bool, EngineError> {
    let Some(module) = self.get(module) else {
      return Err(EngineError::module_not_found(module));
    };
    Ok(module.dirty.swap(false, Ordering::AcqRel))
  }

  /// Replay the tool calls that happened during the given turns and
  /// mutate the GameState.
  pub async fn replay(&self, modules: &HashMap<String, GameplayModule>, turns: &[TurnV2]) -> Result<(), EngineError> {
    let tool_calls = turns
      .iter()
      .flat_map(|turn| &turn.responses)
      .flat_map(|res| &res.tool_calls);
    for tc in tool_calls {
      if tc.name.find("::").is_none() {
        return Err(EngineError::input(format!("invalid tool name {}", tc.name)));
      }

      let mut parts = tc.name.splitn(2, "::");
      let module_name = parts.next().ok_or(EngineError::input("expected module name"))?;
      let tool_name = parts.next().ok_or(EngineError::input("expected tool name"))?;

      let module = modules.get(module_name).ok_or_else(|| {
        EngineError::input(format!("unknown module {module_name} in recorded tool call {}", tc.name))
      })?;
      let ctx = self.view(module_name.to_string());
      let response = module.handle(ctx, tool_name, tc.arguments.clone()).await;
      if response.starts_with("Error:") {
        return Err(EngineError::state("GameState replay encountered an unexpected error"));
      }
    }
    Ok(())
  }

  pub(crate) async fn snapshot(&self) -> Snapshot {
    let mut res: Snapshot = Default::default();
    for (module_id, module_state) in &*self.storage {
      let guard = module_state.storage.read().await;
      res.insert(module_id.clone(), JsonValue::Object(guard.clone()));
    }
    res
  }

  #[inline(always)]
  pub(crate) fn view(&self, module: String) -> GameStateView {
    GameStateView { gamestate: self.clone(), module }
  }

  fn build_storage(map: Snapshot) -> Result<Arc<HashMap<String, ModuleState>>, EngineError> {
    let mut res: HashMap<String, ModuleState> = Default::default();
    for (key, value) in map {
      let value: JsonMap<String, JsonValue> = serde_json::from_value(value)?;
      res.insert(key, ModuleState::new(value));
    }
    Ok(Arc::new(res))
  }
}

#[derive(Debug)]
pub struct ModuleState {
  storage: Arc<RwLock<JsonMap<String, JsonValue>>>,
  dirty: AtomicBool,
}

impl ModuleState {
  pub fn new(storage: JsonMap<String, JsonValue>) -> Self {
    Self {
      storage: Arc::new(RwLock::new(storage)),
      dirty: AtomicBool::new(true),
    }
  }

  /// Get a value from the given keypath and parse it into a `serde` deserializable type.
  pub async fn get<D: DeserializeOwned>(&self, keypath: &[String]) -> Option<Result<D, serde_json::Error>> {
    if keypath.is_empty() {
      return None;
    }

    let guard = self.storage.read().await;
    let mut curr = guard.get(&keypath[0])?;

    for keypart in &keypath[1..] {
      curr = curr.get(keypart)?;
    }
    Some(serde_json::from_value(curr.clone()))
  }

  /// Set a value at the given keypath. Missing keys along the path are
  /// auto-created as JSON objects. Only the module itself is allowed to set
  /// the value, exposed through the [GameStateView] struct.
  async fn set<T: Serialize>(&self, keypath: &[String], value: &T) -> Result<(), EngineError> {
    if keypath.is_empty() {
      return Err(EngineError::input("empty keypath"));
    }

    let value = serde_json::to_value(value)?;
    let mut guard = self.storage.write().await;
    let mut curr: &mut JsonMap<String, JsonValue> = &mut guard;

    for (i, keypart) in keypath.iter().enumerate() {
      if i + 1 == keypath.len() {
        curr.insert(keypart.clone(), value);
        return Ok(());
      }
      if !curr.get(keypart).is_some_and(|v| v.is_object()) {
        curr.insert(keypart.clone(), json!({}));
      }
      curr = curr.get_mut(keypart)
        .and_then(JsonValue::as_object_mut)
        .ok_or_else(|| EngineError::state(format!("not a JSON object at {}", keypath[..=i].join("."))))?;
    }

    unreachable!()
  }
}

impl Default for ModuleState {
  fn default() -> Self {
    Self {
      storage: Default::default(),
      dirty: AtomicBool::new(true),
    }
  }
}

#[derive(Debug, Clone)]
pub struct GameStateView {
  gamestate: GameState,
  module: String,
}

impl GameStateView {
  /// Get & deserialize a value from the given gameplay module at the given keypath.
  pub async fn get<D: DeserializeOwned>(&self, module: &String, keypath: &[String]) -> Option<Result<D, serde_json::Error>> {
    let module = self.gamestate.get(module)?;
    module.get(keypath).await
  }

  /// Serialize & set a value for the current gameplay module at the given keypath.
  pub async fn set<T: Serialize>(&self, keypath: &[String], value: &T) -> Result<(), EngineError> {
    let Some(module) = self.gamestate.get(&self.module) else {
      return Err(EngineError::not_found(format!("Module {} not found", self.module)));
    };
    module.set(keypath, value).await
  }

  #[inline(always)]
  pub fn profile(&self) -> Option<Arc<ProfileV1>> {
    self.gamestate.profile.clone()
  }
}
