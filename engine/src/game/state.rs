use std::collections::HashMap;
use std::sync::Arc;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{Map as JsonMap, Value as JsonValue, json};
use tokio::sync::RwLock;

use crate::game::profile::Profiles;
use crate::config::{Models, NovelCraftConfig};
use crate::error::AppError;
use crate::game::module::{GameplayModule};
use crate::game::pages::{PageV1, Snapshot};

#[derive(Debug, Clone)]
pub struct AppState {
  pub config: Arc<RwLock<NovelCraftConfig>>,
  pub profiles: Arc<RwLock<Profiles>>,
  pub models: Arc<RwLock<Models>>,
}

impl AppState {
  pub async fn init() -> Result<AppState, AppError> {
    let config = NovelCraftConfig::load().await?;
    let profiles = Profiles::load().await?;
    let models = Models::load().await?;

    Ok(Self {
      config: Arc::new(RwLock::new(config)),
      profiles: Arc::new(RwLock::new(profiles)),
      models: Arc::new(RwLock::new(models)),
    })
  }
}

#[derive(Debug, Clone, Default)]
pub struct GameState {
  storage: HashMap<String, ModuleState>,
}

impl GameState {
  pub fn new(snapshot: Snapshot) -> Result<Self, AppError> {
    Ok(Self {
      storage: serde_json::from_value(JsonValue::Object(snapshot))?,
      ..Default::default()
    })
  }

  /// Get a readonly [ModuleState] of the ID'ed module.
  pub fn get(&self, module: &String) -> Option<&ModuleState> {
    self.storage.get(module)
  }

  fn get_mut(&mut self, module: &String) -> Option<&mut ModuleState> {
    self.storage.get_mut(module)
  }

  /// Replay the tool calls that happened during the given pages and
  /// mutate the GameState.
  pub async fn replay(&mut self, modules: &HashMap<String, GameplayModule>, pages: &[PageV1]) -> Result<(), AppError> {
    for page in pages {
      for tc in &page.tool_calls {
        if tc.name.find("::").is_none() {
          return Err(AppError::input(format!("invalid tool name {}", tc.name)));
        }

        let mut parts = tc.name.splitn(2, "::");
        let module_name = parts.next().ok_or(AppError::input("expected module name"))?;
        let tool_name = parts.next().ok_or(AppError::input("expected tool name"))?;

        let module = &modules[module_name];
        let mut ctx = self.view(module_name.to_string());
        let response = module.handle(&mut ctx, tool_name, tc.arguments.clone()).await;
        if response.starts_with("Error:") {
          return Err(AppError::state("GameState replay encountered an unexpected error"));
        }
      }
    }
    Ok(())
  }

  /// Create a new scoped view of this GameState. The holder of this view can mutate
  /// only the given module's state, but read all other modules' states.
  pub(crate) fn view(&mut self, module: String) -> GameStateView<'_> {
    GameStateView {
      gamestate: self,
      module,
    }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModuleState(JsonMap<String, JsonValue>);

impl ModuleState {
  /// Get a value from the given keypath and parse it into a `serde` deserializable type.
  pub fn get<D: DeserializeOwned>(&self, keypath: &[String]) -> Option<Result<D, serde_json::Error>> {
    if keypath.is_empty() {
      return None;
    }

    let Some(mut curr) = self.0.get(&keypath[0]) else { return None };

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
  fn set<T: Serialize>(&mut self, keypath: &[String], value: &T) -> Result<(), AppError> {
    if keypath.is_empty() {
      return Err(AppError::input("empty keypath"));
    }

    let e_not_found = AppError::not_found(format!("Keypath \"{}\"", keypath.join(".")));

    let Some(mut curr) = self.0.get_mut(&keypath[0]) else {
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

#[derive(Debug)]
pub struct GameStateView<'a> {
  gamestate: &'a mut GameState,
  module: String,
}

impl<'a> GameStateView<'a> {
  /// Get & deserialize a value from the given gameplay module at the given keypath.
  pub fn get<D: DeserializeOwned>(&self, module: &String, keypath: &[String]) -> Option<Result<D, serde_json::Error>> {
    self.gamestate
      .get(module)
      .and_then(|module| module.get(keypath))
  }

  /// Serialize & set a value for the current gameplay module at the given keypath.
  pub fn set<T: Serialize>(&mut self, keypath: &[String], value: &T) -> Result<(), AppError> {
    self.gamestate
      .get_mut(&self.module)
      .ok_or(AppError::not_found(format!("Module {} not found", self.module)))?
      .set(keypath, value)
  }
}
