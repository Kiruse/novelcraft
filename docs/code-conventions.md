# Code Conventions

This document describes the code styling, import patterns, and conventions used throughout NovelCraft.

## Rust Conventions

### Formatting

The Rust workspace uses [`rustfmt`](https://rust-lang.github.io/rustfmt/) with a project-level config:

- **Indentation**: 2 spaces (`tab_spaces = 2`)
- Format with `just fmt`, verify with `just fmt-check`

### Engine Command Functions

All command functions are plain `pub async fn` taking `&AppState` as the first parameter (or no state for read-only operations). No `#[tauri::command]` or `#[specta::specta]` decorators.

```rust
use novelcraft_engine::AppState;

pub async fn session_list(app: &AppState) -> Result<Vec<SessionMeta>> {
    // ...
}
```

- Return `Result<T>` using the engine's error type
- Ensure parameter/return types derive `Serialize`/`Deserialize` as needed
- One file per domain in `engine/src/commands/`

### Import Patterns

- **Within engine**: Standard Rust `mod`/`use` paths. No special aliases.
- **GUI depends on engine**: `use novelcraft_engine::commands::...` (or re-exports from `novelcraft_engine::*`).

### Async/Await

Always use `async/await` for async operations:

```rust
let sessions = session::session_list(&app).await?;
session::session_save_meta(&app, id, updated_meta).await?;
```

### Error Handling

Functions return `Result<T>` and use `?` for propagation. Errors are defined in `engine/src/error.rs` using `thiserror`.

```rust
if !session {
    return Err(Error::SessionNotFound(id.to_string()));
}
```

### State Management

`AppState` uses `Arc<Mutex<T>>` pattern — constructed via `AppState::init()`, shared across engine functions by reference (`&AppState`).

```rust
let models = app.models.lock().await;
let config = app.config.lock().await;
```

### LLM Streaming

All LLM calls go through `commands::llm::prompt()`, which takes callback closures:

```rust
commands::llm::prompt(
    &app,
    request,
    OnText { |chunk| /* handle text */ },
    OnReasoning { |chunk| /* handle reasoning */ },
    OnToolCall { |delta| /* handle tool call */ },
    OnError { |msg| /* handle error */ },
    OnDone { |reason, usage| /* handle completion */ },
)?;
```

### Data Persistence

All data access goes through engine command functions — never direct file I/O from the GUI layer.

```rust
use novelcraft_engine::AppState;
use novelcraft_engine::commands::{session, profile, story, lore};

let sessions = session::session_list(&app).await?;
session::session_create(&app, id, story_id, title, description).await?;
session::session_push_page(&app, id, page_entry).await?;
let result = profile::profile_list(&app).await?;
let story = story::story_get(&app, id).await?;
let results = lore::lore_query(&app, id, "search").await?;
```

**No transactions**: Each function call is independent. Consumers drive sequential operations.

### Inspiration REST Client

The Create Story screen's "Get Inspired" data is fetched from a REST inspiration service — GUI-only, with the HTTP plumbing in the reusable `gui/src/api.rs` module (the engine is not involved):

- `gui/src/api.rs`:
  - `const API_BASE_URL: &str = "http://localhost:8000"` — hardcoded base URL (TODO: replace once the service gets a proper domain)
  - `get_json<T: DeserializeOwned>(path, query)` — awaits a `reqwest` GET of `{API_BASE_URL}{path}` with the `query` params (comma-joined values are the caller's choice), applies `error_for_status()`, and decodes the JSON; every failure maps to `GuiError::Api(String)`. Must be called from within an async (tokio) context — the Dioxus runtime provides one
- `gui/src/screens/create_story.rs`:
  - `Inspiration { title, premise, tags }` — screen-local serde type (`tags` holds tag strings)
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