use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use serde::de::DeserializeOwned;
use serde::{Serialize};
use serde_json::{Map as JsonMap, Value as JsonValue, json};
use tokio::sync::RwLock;

use crate::error::AppError;
use crate::game::module::{GameplayModule};
use crate::game::pages::{PageV1, Snapshot};

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
}

impl GameState {
  pub fn new(snapshot: Snapshot) -> Result<Self, AppError> {
    Ok(Self {
      storage: Self::build_storage(snapshot)?,
      ..Default::default()
    })
  }

  /// Get a readonly [ModuleState] of the ID'ed module.
  pub fn get(&self, module: &String) -> Option<&ModuleState> {
    self.storage.get(module)
  }

  /// Mark named module dirty. Errors if no such module exists.
  pub fn mark_dirty(&self, module: &String) -> Result<(), AppError> {
    let Some(module) = self.storage.get(module) else {
      return Err(AppError::module_not_found(module));
    };
    module.dirty.store(true, Ordering::Release);
    Ok(())
  }

  /// Mark named module clean & return whether it *was* dirty. Errors if no
  /// such module exists.
  pub(crate) fn mark_clean(&self, module: &String) -> Result<bool, AppError> {
    let Some(module) = self.get(module) else {
      return Err(AppError::module_not_found(module));
    };
    Ok(module.dirty.swap(false, Ordering::AcqRel))
  }

  /// Replay the tool calls that happened during the given pages and
  /// mutate the GameState.
  pub async fn replay(&self, modules: &HashMap<String, GameplayModule>, pages: &[PageV1]) -> Result<(), AppError> {
    let tool_calls = pages
      .iter()
      .map(|page| &page.responses)
      .flatten()
      .map(|res| &res.tool_calls)
      .flatten();
    for tc in tool_calls {
      if tc.name.find("::").is_none() {
        return Err(AppError::input(format!("invalid tool name {}", tc.name)));
      }

      let mut parts = tc.name.splitn(2, "::");
      let module_name = parts.next().ok_or(AppError::input("expected module name"))?;
      let tool_name = parts.next().ok_or(AppError::input("expected tool name"))?;

      let module = &modules[module_name];
      let ctx = self.view(module_name.to_string());
      let response = module.handle(ctx, tool_name, tc.arguments.clone()).await;
      if response.starts_with("Error:") {
        return Err(AppError::state("GameState replay encountered an unexpected error"));
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

  fn build_storage(map: Snapshot) -> Result<Arc<HashMap<String, ModuleState>>, AppError> {
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
    let Some(mut curr) = guard.get(&keypath[0]) else { return None };

    for keypart in &keypath[1..] {
      if let Some(value) = curr.get(keypart) {
        curr = value;
      } else {
        return None;
      }
    }
    return Some(serde_json::from_value(curr.clone()));
  }

  /// Set a value at the given keypath. Only the module itself is allowed to set the value,
  /// exposed through the [GameStateView] struct.
  async fn set<T: Serialize>(&self, keypath: &[String], value: &T) -> Result<(), AppError> {
    if keypath.is_empty() {
      return Err(AppError::input("empty keypath"));
    }

    let e_not_found = AppError::not_found(format!("Keypath \"{}\"", keypath.join(".")));
    let mut guard = self.storage.write().await;

    let Some(mut curr) = guard.get_mut(&keypath[0]) else {
      return Err(e_not_found);
    };

    for (i, keypart) in keypath[1..keypath.len() - 1].iter().enumerate() {
      if !curr.is_object() {
        return Err(AppError::state(format!("not a JSON object at {}", keypath[0..i].join("."))));
      }
      curr = curr.as_object_mut()
        .unwrap()
        .entry(keypart)
        .or_insert(json!({}));
    }

    curr[keypath.last().unwrap()] = serde_json::to_value(value)?;
    Ok(())
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
    let Some(module) = self.gamestate.get(module) else { return None };
    module.get(keypath).await
  }

  /// Serialize & set a value for the current gameplay module at the given keypath.
  pub async fn set<T: Serialize>(&self, keypath: &[String], value: &T) -> Result<(), AppError> {
    let Some(module) = self.gamestate.get(&self.module) else {
      return Err(AppError::not_found(format!("Module {} not found", self.module)));
    };
    module.set(keypath, value).await
  }
}
