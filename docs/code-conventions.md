# Code Conventions

This document describes the code styling, import patterns, and conventions used throughout NovelCraft.

## Rust Conventions

### Formatting

Formatting is **manual** — **never run `cargo fmt`, `just fmt`, or `just fmt-check`**. rustfmt is not well suited for this codebase's UI code (rsx blocks & long attribute chains). The indentation convention is 2 spaces (`engine/src/rustfmt.toml` sets `tab_spaces = 2`); match the style of surrounding code by hand.

### Engine Methods

Engine functionality lives as methods on `NovelCraftEngine` (`engine/src/game/engine.rs`) — or on `SessionV2`/`GameState` when it belongs at that layer. There is no `AppState` and no `commands/` module tree.

```rust
use novelcraft_engine::game::session::SessionV2;

let sessions = NovelCraftEngine::list_sessions().await?;
let mut session = SessionV2::load(id, &profiles).await?;
session.fork(turn_index, &profiles).await?;
```

- Return `Result<T>` using the engine's error type (`EngineError`)
- Types crossing the GUI boundary derive `Serialize`/`Deserialize` as needed
- Game-engine code is grouped by layer: `game/engine.rs`, `game/session.rs`, `game/turns.rs`, `game/state.rs`, `game/module/*`

### Import Patterns

- **Within engine**: Standard Rust `mod`/`use` paths. No special aliases.
- **GUI depends on engine**: `use novelcraft_engine::game::session::SessionV2;`, `use novelcraft_engine::config::NovelCraftConfig;`, etc. Screens never import engine types directly — data flows through `gui/src/engine.rs` (bridge) and `gui/src/state.rs` (signals).

### Async/Await

Always use `async/await` for async operations:

```rust
let sessions = NovelCraftEngine::list_sessions().await?;
session.save().await?;
```

### Error Handling

Functions return `Result<T>` and use `?` for propagation. Errors are defined in `engine/src/error.rs` using `thiserror` (`EngineError`, with helpers like `EngineError::state`, `EngineError::validation`); the GUI wraps them in `GuiError` (`gui/src/error.rs`).

```rust
if session.turn_count() == 0 {
    return Err(EngineError::state("fresh session"));
}
```

### State Management

There is no `AppState`. The engine is a single `NovelCraftEngine` value owning the config and the active session (`Option<SessionV2>`). The GUI holds it process-wide behind `OnceLock<Arc<tokio::sync::Mutex<NovelCraftEngine>>>` (`gui/src/engine.rs`); all access locks the mutex — operations are serialized.

```rust
let engine = engine();
let mut eng = engine.lock().await;
eng.set_config(config);
```

### LLM Streaming

All LLM calls go through the engine, which runs the kiruklaw `AgentLoop` (HTTP streaming + SSE parsing live inside `kiruklaw-agent-loop`). Chunks stream to the caller through a tokio mpsc channel of `AgentMessageChunk`s:

```rust
let (tx, mut rx) = tokio::sync::mpsc::channel::<AgentMessageChunk>(100);
engine.prompt(content, tx).await?;

while let Some(chunk) = rx.recv().await {
    match chunk {
        AgentMessageChunk::Content { text } => { /* append text */ }
        AgentMessageChunk::ToolCallStart { .. } | AgentMessageChunk::ToolCallArgs { .. } => { /* activity */ }
        AgentMessageChunk::Reasoning { .. } | AgentMessageChunk::Done { .. } => { /* ignore */ }
    }
}
```

Memory compression additionally uses a one-shot `kiruklaw_agent_loop::prompt` call (no agent loop, no tools) with the `Summarizer` model and `SUMMARIZER_PERSONA` (`engine/src/game/module/memory.rs`).

### Data Persistence

All data access goes through engine types (`NovelCraftEngine`, `SessionV2`) — never direct file I/O from the GUI layer. Path resolution is centralized in `engine/src/paths.rs`.

```rust
use novelcraft_engine::game::engine::NovelCraftEngine;
use novelcraft_engine::game::session::SessionV2;

let sessions = NovelCraftEngine::list_sessions().await?;   // associated fn: metadata only, newest first
let mut engine = NovelCraftEngine::new(config);
let session = engine.create_session(title, exposition).await?; // &mut self
let mut session = SessionV2::load(id, &profiles).await?;
session.save().await?;                                     // meta + snapshot + history
```

**No transactions**: each save writes whole files. Consumers drive sequential operations.

### Inspiration REST Client

The Create Story screen's "Get Inspired" data is fetched from a REST inspiration service — GUI-only, with the HTTP plumbing in the reusable `gui/src/api.rs` module (the engine is not involved):

- `gui/src/api.rs`:
  - `const API_BASE_URL: &str = "http://localhost:8000"` — hardcoded base URL (TODO: replace once the service gets a proper domain)
  - `get_json<T: DeserializeOwned>(path, query)` — awaits a `reqwest` GET of `{API_BASE_URL}{path}` with the `query` params (comma-joined values are the caller's choice), applies `error_for_status()`, and decodes the JSON; every failure maps to `GuiError::Api(String)`. Must be called from within an async (tokio) context — the Dioxus runtime provides one
- `gui/src/screens/create_story.rs`:
  - `Inspiration { title, premise, tags }` — screen-local serde type (`tags` holds tag strings); `premise` is markdown, rendered with the shared `Markdown` component
  - `GET /inspirations/tags` → `Vec<String>` (plain tags; rendered as-is for now — formatting/localization is the GUI's concern, not implemented yet) and `GET /inspirations?tags=a,b` → `Vec<Inspiration>`
  - `fetch_tags()` / `fetch_inspirations(selected)` — wrappers over `api::get_json`; `fetch_inspirations` passes the selected tags comma-joined as the `tags` query param — the service OR-filters server-side (an empty selection omits the param, matching everything)

`reqwest = { version = "0.12", features = ["json"] }` is a GUI dependency for this client (streaming is not used).

```rust
let tags = fetch_tags().await?;
let inspirations = fetch_inspirations(&selected).await?;
```

## Logging

The `log` crate is a workspace dependency used by both the engine and GUI crates. The GUI logs via `log::error!`/`log::warn!` (import them explicitly — `use log::error;` — a glob `use log::*` collides with the Dioxus prelude); user-facing failures are surfaced with `push_toast(Toast::error(...))` instead.

## Dioxus Conventions

### Routing & Navigation

Navigation is a typed route enum (`gui/src/screens/mod.rs`) handled by the Dioxus Router:

```rust
#[derive(Clone, Routable, Debug, PartialEq)]
pub enum Route {
  #[route("/")]
  Home {},
  // ...
  #[route("/story/:id")]
  StoryOverview { id: String },
}
```

Navigate with `navigator().push(Route::...)`. Back navigation goes through `go_back(from: Route)` — gameplay returns to the story overview, everything else home. Always terminate `push(...)` inside event handlers with a semicolon: non-unit returns are interpreted as async spawns and fail to compile.

### State

- Component-local state: `use_signal` (needs `let mut` when mutated). Mount-once side effects: `use_hook(|| ...)` + `spawn`.
- Cross-screen state: `GlobalSignal`s in `gui/src/state.rs`, wrapped in `Query<T>` (Empty/Error/Stale/Recent). Never touch global signals from `main()` — signal access requires the Dioxus runtime.
- Clone values out of `.read()` guards before capturing them in `'static` event closures.

### Styling

Styling is Tailwind CSS v4, written as `class` strings directly in `rsx!`:

```rust
button {
  class: "w-full p-3 flex flex-col gap-1 border rounded-sm border-nc-border cursor-pointer hover:border-nc-label",
  onclick: move |_| { ... },
  "label"
}
```

Theme colors are `@theme` tokens in `gui/input.css` (`nc-bg`, `nc-text`, `nc-label`, `nc-border`, `nc-danger`, `nc-dangerfg`, `nc-success`, `nc-warn`), compiled into utilities like `bg-nc-bg` / `text-nc-label` by the Tailwind CLI (`just css`; keep the watcher running under `just dev`).

### Event Handlers

```rust
// Plain handler
onclick: move |_| { nav.push(Route::CreateStory {}); }

// With a Signal captured (Copy — no clone needed)
onclick: move |_| selected.write().insert(tag.clone()),

// Keyboard events expose .data (key, modifiers) and prevent_default
onkeydown: move |ev: KeyboardEvent| {
  if ev.data.modifiers().contains(Modifiers::ALT) && ev.data.key() == Key::ArrowLeft {
    ev.prevent_default();
  }
},

// Controlled text input
input {
  class: INPUT_CLASS,
  value: "{title}",
  oninput: move |ev: FormEvent| title.set(ev.value()),
}
```

## Related Documentation

- [Project Structure](./project-structure.md) - File organization and directory layout
- [Data Storage](./database-schema.md) - JSON file formats and storage layout
- [Engine API](./api-routes.md) - Engine command function reference
- [GUI Architecture](./gui-architecture.md) - Dioxus components, screens, styling conventions