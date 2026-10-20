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
  ///
  /// Tool calls whose *recorded* response is an error are skipped: they had
  /// no state effect when they originally ran (the agent retried with a
  /// corrected call), and re-dispatching them could double-apply an action
  /// if validation semantics have changed since. Calls that succeeded
  /// originally are re-dispatched; if a re-dispatch errors anyway (state
  /// drift), the mismatch is logged and replay continues — an error here
  /// must never block loading the session.
  ///
  /// After each turn's tool traffic, that turn's manual edits
  /// ([TurnV2::edits]) are applied — the player made them against the state
  /// as it stood at that page.
  pub async fn replay(&self, modules: &HashMap<String, GameplayModule>, turns: &[TurnV2]) -> Result<(), EngineError> {
    for turn in turns {
      for res in &turn.responses {
        let responses: HashMap<&str, &String> =
          res.tool_responses.iter().map(|(id, resp)| (id.as_str(), resp)).collect();

        for tc in &res.tool_calls {
          if responses.get(tc.id.as_str()).is_some_and(|resp| resp.starts_with("Error:")) {
            log::warn!("replay: skipping error'ed tool call {}", tc.name);
            continue;
          }

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
            log::warn!("replay: re-dispatch of originally successful call {} errored: {response}", tc.name);
          }
        }
      }

      for (keypath, value) in &turn.edits {
        let Some((module_name, path)) = keypath.split_first() else {
          log::warn!("replay: skipping empty edit keypath");
          continue;
        };
        if !modules.contains_key(module_name) {
          log::warn!("replay: skipping manual edit for unknown module {module_name}");
          continue;
        }
        let ctx = self.view(module_name.clone());
        if let Err(err) = ctx.apply_edit(path, value).await {
          log::warn!("replay: failed to apply manual edit {keypath:?}: {err}");
        }
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

  /// Get the entire module state object parsed into a `serde` deserializable type.
  pub async fn get_all<D: DeserializeOwned>(&self) -> Result<D, serde_json::Error> {
    let guard = self.storage.read().await;
    serde_json::from_value(JsonValue::Object(guard.clone()))
  }

  /// Replace the entire module state object and mark the module dirty.
  pub async fn replace(&self, storage: JsonMap<String, JsonValue>) {
    *self.storage.write().await = storage;
    self.dirty.store(true, Ordering::Release);
  }

  /// Remove the value at the given keypath. Missing keys are ignored.
  pub async fn remove(&self, keypath: &[String]) -> Result<(), EngineError> {
    if keypath.is_empty() {
      return Err(EngineError::input("empty keypath"));
    }

    let mut guard = self.storage.write().await;
    let mut curr: &mut JsonMap<String, JsonValue> = &mut guard;
    for (i, keypart) in keypath[..keypath.len() - 1].iter().enumerate() {
      curr = curr.get_mut(keypart)
        .and_then(JsonValue::as_object_mut)
        .ok_or_else(|| EngineError::state(format!("not a JSON object at {}", keypath[..=i].join("."))))?;
    }
    curr.remove(keypath.last().expect("non-empty keypath"));
    Ok(())
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

  /// Remove the value at the given keypath within this module's state.
  pub async fn remove(&self, keypath: &[String]) -> Result<(), EngineError> {
    let Some(module) = self.gamestate.get(&self.module) else {
      return Err(EngineError::not_found(format!("Module {} not found", self.module)));
    };
    module.remove(keypath).await
  }

  /// Apply a manual state edit at the given module-relative keypath: set
  /// the string `value`, or remove the field when `value` is empty. Decimal
  /// segments index into arrays. Used for manual edits recorded on
  /// [TurnV2::edits] and re-applied by [GameState::replay].
  pub(crate) async fn apply_edit(&self, keypath: &[String], value: &str) -> Result<(), EngineError> {
    if keypath.is_empty() {
      return Err(EngineError::input("empty keypath"));
    }
    let Some(module) = self.gamestate.get(&self.module) else {
      return Err(EngineError::not_found(format!("Module {} not found", self.module)));
    };

    let map: JsonMap<String, JsonValue> = module.get_all().await.map_err(|err| EngineError::parse(err.to_string()))?;
    let mut state = JsonValue::Object(map);
    let cursor = descend_edit_path(&mut state, &keypath[..keypath.len() - 1])?;
    let last = keypath.last().expect("non-empty keypath");
    if value.is_empty() {
      match cursor {
        JsonValue::Array(items) => {
          if let Ok(idx) = last.parse::<usize>() {
            if idx < items.len() {
              items.remove(idx);
            }
          }
        }
        JsonValue::Object(map) => {
          map.remove(last);
        }
        _ => {}
      }
    } else {
      *descend_edit_path(cursor, std::slice::from_ref(last))? = JsonValue::String(value.to_string());
    }

    let JsonValue::Object(storage) = state else { unreachable!("state starts as an object") };
    module.replace(storage).await;
    Ok(())
  }

  #[inline(always)]
  pub fn profile(&self) -> Option<Arc<ProfileV1>> {
    self.gamestate.profile.clone()
  }
}

/// Walk (and auto-create) containers along a keypath within a JSON value.
/// Decimal segments index into arrays; missing containers materialize as
/// empty objects — or empty arrays when the *next* segment is numeric.
fn descend_edit_path<'a>(cursor: &'a mut JsonValue, keypath: &[String]) -> Result<&'a mut JsonValue, EngineError> {
  let mut cursor = cursor;
  for (i, segment) in keypath.iter().enumerate() {
    let wants_array = keypath.get(i + 1).is_some_and(|next| next.parse::<usize>().is_ok());
    match cursor {
      JsonValue::Array(items) => {
        let idx: usize = segment
          .parse()
          .map_err(|_| EngineError::input(format!("invalid array index {segment}")))?;
        while items.len() <= idx {
          items.push(JsonValue::Null);
        }
        cursor = &mut items[idx];
      }
      JsonValue::Object(map) => {
        cursor = map.entry(segment.clone()).or_insert_with(|| {
          if wants_array {
            json!([])
          } else {
            json!({})
          }
        });
      }
      JsonValue::Null => {
        *cursor = if wants_array {
          json!([])
        } else {
          json!({})
        };
        cursor = descend_edit_path(cursor, &keypath[i..])?;
        break;
      }
      _ => {
        return Err(EngineError::state(format!(
          "not a container at {}",
          keypath[..=i].join(".")
        )));
      }
    }
  }
  Ok(cursor)
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::game::module::GameplayModule;
  use crate::game::turns::{AgentResponse, TurnV2};

  fn modules() -> HashMap<String, GameplayModule> {
    GameplayModule::default_set().into_iter().collect()
  }

  fn call(id: &str, name: &str, arguments: &str) -> kiruklaw_agent_loop::ToolCall {
    kiruklaw_agent_loop::ToolCall {
      id: id.to_string(),
      name: name.to_string(),
      arguments: arguments.to_string(),
    }
  }

  fn turn(calls: Vec<kiruklaw_agent_loop::ToolCall>, responses: Vec<(String, String)>) -> TurnV2 {
    TurnV2 {
      prompt: Some("go".to_string()),
      responses: vec![AgentResponse {
        tool_calls: calls,
        tool_responses: responses,
        ..Default::default()
      }],
      ..Default::default()
    }
  }

  async fn timeline(state: &GameState) -> Option<String> {
    state.view("story".to_string())
      .get::<String>(&"story".to_string(), &["timeline".to_string()])
      .await
      .transpose()
      .unwrap()
  }

  #[tokio::test]
  async fn replay_applies_successful_calls() {
    let state = GameState::new(Default::default(), vec!["story".to_string()], None).unwrap();
    let turn = turn(
      vec![call("t1", "story::manage_timeline", r#"{"contents":"[ ] find a map"}"#)],
      vec![("t1".to_string(), "ok".to_string())],
    );

    state.replay(&modules(), &[turn]).await.unwrap();

    assert!(timeline(&state).await.unwrap().contains("find a map"));
  }

  #[tokio::test]
  async fn replay_skips_errored_calls() {
    // The call would SUCCEED under the current (permissive) timeline parser —
    // but its recorded response is an error, so it had no effect originally
    // and must not be resurrected.
    let state = GameState::new(Default::default(), vec!["story".to_string()], None).unwrap();
    let turn = turn(
      vec![call("t1", "story::manage_timeline", r#"{"contents":"- find a map"}"#)],
      vec![("t1".to_string(), "Error: invalid todo item".to_string())],
    );

    state.replay(&modules(), &[turn]).await.unwrap();

    assert!(timeline(&state).await.is_none());
  }

  #[tokio::test]
  async fn replay_tolerates_redispatch_errors() {
    // Originally succeeded, but the re-dispatch now errors (state drift) —
    // replay must log and continue, never block loading.
    let state = GameState::new(Default::default(), vec!["story".to_string()], None).unwrap();
    let turn = turn(
      vec![
        call("t1", "story::manage_timeline", r#"{"contents":"[ ] find a map"}"#),
        call("t2", "story::manage_timeline", r#"{"contents":"[x] find a map"}"#),
        call("t3", "story::manage_timeline", r#"{"contents":"[ ] find a map"}"#),
      ],
      vec![
        ("t1".to_string(), "ok".to_string()),
        ("t2".to_string(), "ok".to_string()),
        // re-checking an already checked item is illegal; recorded as success
        // anyway (drift) — replay must not fail the load over it.
        ("t3".to_string(), "ok".to_string()),
      ],
    );

    state.replay(&modules(), &[turn]).await.unwrap();
  }

  #[tokio::test]
  async fn replay_applies_manual_edits() {
    let state = GameState::new(
      Default::default(),
      vec!["story".to_string(), "npcs".to_string()],
      None,
    ).unwrap();
    let mut turn = turn(
      vec![call("t1", "story::manage_timeline", r#"{"contents":"[ ] find a map"}"#)],
      vec![("t1".to_string(), "ok".to_string())],
    );
    turn.edits = vec![
      (vec!["story".to_string(), "timeline".to_string()], "- [x] find a map".to_string()),
      (vec!["npcs".to_string(), "Mira".to_string(), "mood".to_string()], "curious".to_string()),
    ];

    state.replay(&modules(), &[turn]).await.unwrap();

    // Edits apply AFTER the turn's tool calls.
    assert_eq!(timeline(&state).await.unwrap(), "- [x] find a map");
    let mood = state.view("npcs".to_string())
      .get::<String>(&"npcs".to_string(), &["Mira".to_string(), "mood".to_string()])
      .await
      .transpose()
      .unwrap();
    assert_eq!(mood.as_deref(), Some("curious"));
  }

  #[tokio::test]
  async fn apply_edit_sets_removes_and_indexes_arrays() {
    let state = GameState::new(
      Default::default(),
      vec!["story".to_string(), "npcs".to_string()],
      None,
    ).unwrap();
    let view = state.view("story".to_string());

    view.apply_edit(&["timeline".to_string()], "first").await.unwrap();
    view.apply_edit(
      &["chapters".to_string(), "0".to_string(), "title".to_string()],
      "The Road",
    ).await.unwrap();
    // Empty value removes the field.
    view.apply_edit(&["timeline".to_string()], "").await.unwrap();

    let map: JsonMap<String, JsonValue> = state.get(&"story".to_string())
      .unwrap()
      .get_all()
      .await
      .unwrap();
    assert!(map.get("timeline").is_none());
    assert_eq!(map["chapters"][0]["title"], "The Road");
  }

  #[tokio::test]
  async fn replay_tolerates_bad_edits() {
    let state = GameState::new(Default::default(), vec!["story".to_string()], None).unwrap();
    let mut turn = turn(vec![], vec![]);
    turn.edits = vec![
      // Unknown module — skipped with a warning, not fatal.
      (vec!["ghost".to_string(), "key".to_string()], "x".to_string()),
      // Valid edit still applies.
      (vec!["story".to_string(), "timeline".to_string()], "- [ ] done".to_string()),
    ];

    state.replay(&modules(), &[turn]).await.unwrap();
    assert_eq!(timeline(&state).await.unwrap(), "- [ ] done");
  }

  #[tokio::test]
  async fn replay_errors_on_unknown_module() {
    let state = GameState::new(Default::default(), vec!["story".to_string()], None).unwrap();
    let turn = turn(
      vec![call("t1", "memory::note_events", "{}")],
      vec![("t1".to_string(), "ok".to_string())],
    );

    assert!(state.replay(&modules(), &[turn]).await.is_err());
  }
}
