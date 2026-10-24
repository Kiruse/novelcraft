//! Generalized sidebar rendering of gameplay module contexts.
//!
//! [`ModuleContext`] is implemented per gameplay module to render its
//! sidebar section and to handle user edits to the module's state. The
//! [`DefaultModuleContext`] fallback renders any module's state as a tree
//! of labeled, editable leaves — so optional future modules show up in the
//! sidebar without extra work.

use dioxus::prelude::*;
use serde_json::Value;

use novelcraft_engine::game::module::story::StoryModule;

/// A path to a node within a module's state tree: object keys, with decimal
/// array indices for array containers.
pub type StatePath = Vec<String>;

/// Per-leaf editing state: the path of the leaf being edited + its draft.
pub type EditingLeaf = Option<(StatePath, String)>;

/// GUI-side renderer of a gameplay module's sidebar context, including
/// handling of user edits to the module's state.
pub trait ModuleContext {
  /// Sidebar section title.
  fn title(&self) -> String;
  /// Whether the section starts expanded.
  fn default_open(&self) -> bool {
    false
  }
  /// Whether the module's state contributes any content to the sidebar.
  fn has_content(&self, state: &Value) -> bool {
    state.as_object().is_some_and(|map| !map.is_empty())
  }
  /// Apply a leaf edit to the state tree, returning the updated tree.
  fn edit(&self, state: &Value, path: &StatePath, value: String) -> Value {
    set_path(state, path, Value::String(value))
  }
  /// Render the module context. Leaf edits flow through `editing` (draft
  /// state) and `on_commit` (applied via [`Self::edit`] and persisted by
  /// the caller). When `editable` is false, leaves render as static text.
  fn render(
    &self,
    state: &Value,
    editable: bool,
    editing: Signal<EditingLeaf>,
    on_commit: EventHandler<(StatePath, String)>,
  ) -> Element;
}

/// Resolve the sidebar renderer for a gameplay module ID. Custom
/// implementations take precedence; unknown modules fall back to the
/// generic tree editor.
pub fn module_context(module_id: &str) -> Box<dyn ModuleContext> {
  match module_id {
    StoryModule::ID => Box::new(StoryModuleContext),
    other => Box::new(DefaultModuleContext::new(other)),
  }
}

// ── Default tree renderer ────────────────────────────────────────────────────

/// Fallback renderer: shows the module state as a tree of labeled,
/// editable leaves.
pub struct DefaultModuleContext {
  title: String,
}

impl DefaultModuleContext {
  pub fn new(module_id: &str) -> Self {
    let title = match module_id {
      "npcs" => "NPCs".to_string(),
      "locations" => "Locations".to_string(),
      "player" => "Player".to_string(),
      "story" => "Story".to_string(),
      other => other.to_string(),
    };
    Self { title }
  }
}

impl ModuleContext for DefaultModuleContext {
  fn title(&self) -> String {
    self.title.clone()
  }

  fn render(
    &self,
    state: &Value,
    editable: bool,
    editing: Signal<EditingLeaf>,
    on_commit: EventHandler<(StatePath, String)>,
  ) -> Element {
    let entries: Vec<(String, &Value)> = state
      .as_object()
      .map(|map| map.iter().map(|(key, value)| (key.clone(), value)).collect())
      .unwrap_or_default();

    rsx! {
      div { class: "flex flex-col gap-3 text-sm",
        for (key, child) in entries {
          { render_node(child, key.clone(), vec![key], editable, editing, on_commit) }
        }
      }
    }
  }
}

/// Extend a state path by one segment.
fn child_path(path: &StatePath, segment: impl Into<String>) -> StatePath {
  let mut path = path.clone();
  path.push(segment.into());
  path
}

fn render_node(
  value: &Value,
  key: String,
  path: StatePath,
  editable: bool,
  editing: Signal<EditingLeaf>,
  on_commit: EventHandler<(StatePath, String)>,
) -> Element {
  match value {
    Value::Object(map) => rsx! {
      div { class: "flex flex-col gap-1", key: "{key}",
        span { class: "text-nc-label text-xs", { key } }
        div { class: "flex flex-col gap-1 pl-3",
          for (child_key, child) in map {
            { render_node(child, child_key.clone(), child_path(&path, child_key.clone()), editable, editing, on_commit) }
          }
        }
      }
    },
    Value::Array(items) => rsx! {
      div { class: "flex flex-col gap-1", key: "{key}",
        span { class: "text-nc-label text-xs", { key } }
        div { class: "flex flex-col gap-1 pl-3",
          for (idx, child) in items.iter().enumerate() {
            { render_node(child, format!("[{idx}]"), child_path(&path, idx.to_string()), editable, editing, on_commit) }
          }
        }
      }
    },
    _ => rsx! {
      div { class: "flex flex-col gap-0.5", key: "{key}",
        span { class: "text-nc-label text-xs", { key } }
        { editable_leaf(path, value, editable, editing, on_commit) }
      }
    },
  }
}

/// An editable leaf: rendered as text until clicked, then a textarea that
/// commits on blur. When `editable` is false, renders as static text.
pub fn editable_leaf(
  path: StatePath,
  value: &Value,
  editable: bool,
  mut editing: Signal<EditingLeaf>,
  on_commit: EventHandler<(StatePath, String)>,
) -> Element {
  let text = match value {
    Value::String(s) => s.clone(),
    Value::Null => String::new(),
    other => other.to_string(),
  };

  if editable && editing.read().as_ref().is_some_and(|(p, _)| *p == path) {
    let draft = editing.read().as_ref().map(|(_, v)| v.clone()).unwrap_or_default();
    let original = text.clone();
    rsx! {
      textarea {
        class: "w-full bg-transparent resize-none px-2 py-1 border rounded-sm border-nc-border text-nc-text focus:border-nc-label focus:outline-none whitespace-pre-wrap",
        rows: "{draft.lines().count().max(1)}",
        value: "{draft}",
        onmounted: move |ev: MountedEvent| async move {
          let _ = ev.data.set_focus(true).await;
        },
        oninput: move |ev: FormEvent| {
          if let Some((_, draft)) = editing.write().as_mut() {
            *draft = ev.value();
          }
        },
        onblur: move |_| {
          let Some((path, draft)) = editing.write().take() else { return };
          if draft != original {
            on_commit.call((path, draft));
          }
        },
      }
    }
  } else {
    let click_text = text.clone();
    let empty = text.trim().is_empty();
    let leaf_class = if !editable {
      // Static text — the sidebar only edits on the last page.
      "whitespace-pre-wrap break-words min-h-4"
    } else if empty {
      "cursor-text rounded-sm hover:bg-nc-text/5 whitespace-pre-wrap break-words min-h-4 text-nc-text/40"
    } else {
      "cursor-text rounded-sm hover:bg-nc-text/5 whitespace-pre-wrap break-words min-h-4"
    };
    rsx! {
      div {
        class: leaf_class,
        onclick: move |_| editing.set(Some((path.clone(), click_text.clone()))),
        { if empty { "(empty)".to_string() } else { text } }
      }
    }
  }
}

/// Set the leaf at `path` within a state tree, treating decimal segments as
/// array indices where the container is an array. Clones on write.
pub fn set_path(state: &Value, path: &[String], value: Value) -> Value {
  let Some(first) = path.first() else {
    return value;
  };
  match state {
    // Missing intermediate objects materialize as empty containers.
    Value::Null => set_path(&Value::Object(Default::default()), path, value),
    Value::Array(items) => {
      let idx: usize = first.parse().unwrap_or(usize::MAX);
      let mut items = items.clone();
      match items.get_mut(idx) {
        None => {}
        Some(item) if path.len() == 1 => *item = value,
        Some(item) => *item = set_path(item, &path[1..], value),
      }
      Value::Array(items)
    }
    Value::Object(map) => {
      let mut map = map.clone();
      if path.len() == 1 {
        map.insert(first.clone(), value);
      } else {
        let child = map.get(first).cloned().unwrap_or(Value::Null);
        map.insert(first.clone(), set_path(&child, &path[1..], value));
      }
      Value::Object(map)
    }
    _ => value,
  }
}

// ── Story module ─────────────────────────────────────────────────────────────

/// Custom renderer for the story module: timeline, rolling summary &
/// agent-maintained chapters, with editable leaves.
pub struct StoryModuleContext;

impl ModuleContext for StoryModuleContext {
  fn title(&self) -> String {
    "Story".to_string()
  }

  fn default_open(&self) -> bool {
    true
  }

  fn has_content(&self, state: &Value) -> bool {
    !state.get("timeline").and_then(Value::as_str).unwrap_or_default().trim().is_empty()
      || !state.get("summary").and_then(Value::as_str).unwrap_or_default().trim().is_empty()
      || state.get("chapters").and_then(Value::as_array).is_some_and(|chapters| {
        chapters.iter().any(|chapter| {
          !chapter.get("title").and_then(Value::as_str).unwrap_or_default().trim().is_empty()
            || !chapter.get("notes").and_then(Value::as_str).unwrap_or_default().trim().is_empty()
        })
      })
  }

  fn render(
    &self,
    state: &Value,
    editable: bool,
    editing: Signal<EditingLeaf>,
    on_commit: EventHandler<(StatePath, String)>,
  ) -> Element {
    let timeline = state.get("timeline").and_then(Value::as_str).unwrap_or_default();
    let summary = state.get("summary").and_then(Value::as_str).unwrap_or_default();
    let chapters = state.get("chapters").and_then(Value::as_array).cloned().unwrap_or_default();

    rsx! {
      div { class: "flex flex-col gap-3 text-sm",
        div { class: "flex flex-col gap-1",
          div { class: "text-nc-label", "Timeline" }
          { editable_leaf(vec!["timeline".to_string()], &Value::String(timeline.to_string()), editable, editing, on_commit) }
        }
        div { class: "flex flex-col gap-1",
          div { class: "text-nc-label", "Summary" }
          { editable_leaf(vec!["summary".to_string()], &Value::String(summary.to_string()), editable, editing, on_commit) }
        }
        div { class: "flex flex-col gap-2",
          div { class: "text-nc-label", "Chapters" }
          for (idx, chapter) in chapters.iter().enumerate() {
            {
              let title = chapter.get("title").and_then(Value::as_str).unwrap_or_default();
              let notes = chapter.get("notes").and_then(Value::as_str).unwrap_or_default();
              rsx! {
                div { class: "flex flex-col gap-1", key: "{idx}",
                  span { class: "text-nc-label text-xs", "Chapter {idx + 1}" }
                  { editable_leaf(
                      vec!["chapters".to_string(), idx.to_string(), "title".to_string()],
                      &Value::String(title.to_string()),
                      editable,
                      editing,
                      on_commit,
                  ) }
                  { editable_leaf(
                      vec!["chapters".to_string(), idx.to_string(), "notes".to_string()],
                      &Value::String(notes.to_string()),
                      editable,
                      editing,
                      on_commit,
                  ) }
                }
              }
            }
          }
        }
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use serde_json::json;

  #[test]
  fn set_path_sets_object_leaves() {
    let state = json!({"summary": "old", "chapters": []});
    let updated = set_path(&state, &["summary".to_string()], Value::String("new".into()));
    assert_eq!(updated, json!({"summary": "new", "chapters": []}));
  }

  #[test]
  fn set_path_indexes_arrays() {
    let state = json!({"chapters": [{"title": "a", "notes": "b"}, {"title": "c", "notes": "d"}]});
    let updated = set_path(
      &state,
      &["chapters".to_string(), "1".to_string(), "notes".to_string()],
      Value::String("edited".into()),
    );
    assert_eq!(
      updated,
      json!({"chapters": [{"title": "a", "notes": "b"}, {"title": "c", "notes": "edited"}]})
    );
  }

  #[test]
  fn set_path_creates_missing_objects_and_ignores_bad_indices() {
    let state = json!({});
    let updated = set_path(&state, &["npcs".to_string(), "Mira".to_string()], Value::String("x".into()));
    assert_eq!(updated, json!({"npcs": {"Mira": "x"}}));

    // Out-of-bounds array indices are ignored.
    let state = json!({"items": ["a"]});
    let updated = set_path(&state, &["items".to_string(), "5".to_string()], Value::String("x".into()));
    assert_eq!(updated, json!({"items": ["a"]}));
  }
}
