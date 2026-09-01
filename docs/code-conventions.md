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

## Logging

The `log` crate is a workspace dependency used by both the engine and GUI crates.

### GUI Logging Utilities (`gui/src/util.rs`)

`gui/src/util.rs` defines a `Loggable` trait and a `LogLevel` enum for ergonomic logging in the GUI layer.

**`LogLevel` enum** — mirrors `log::Level` with variants: `Trace`, `Debug`, `Info`, `Warn`, `Error`. Converts to `log::Level` via `From<LogLevel> for log::Level`.

**`Loggable` trait** — required method `log(&self, level: LogLevel)`, with default convenience methods `trace()`, `debug()`, `info()`, `warn()`, `error()` that delegate to `log()` with the corresponding level.

**Blanket `impl<T, E: Display> Loggable for Result<T, E>`** — logs `"Ok"` or `"Err: {e}"` at the specified level. Useful for ad-hoc result inspection:

```rust
use crate::util::Loggable;

some_operation().info();     // logs "Ok" at Info level
fallible_op().error();      // logs "Err: timeout" at Error level
```

## GPUI Conventions

### Screen-Based Navigation

Navigation uses an enum-based state machine in the root view (`AppRoot`). Screens are stateless rendering functions returning `Div` elements.

```rust
pub enum Screen {
  Gameplay,
  Home(home::HomeData),
  Settings(Theme),
  Story,
}
```

### Styling

All styling uses gpui's `Styled` trait (Tailwind-like API):

```rust
.cursor_pointer()
.hover(|style| style.opacity(0.7))
.relative().top_4().right_4()
```

### Event Handlers

```rust
// On InteractiveElement (no id required)
.on_mouse_down(MouseButton::Left, |ev, window, cx| { ... })

// On StatefulInteractiveElement (requires .id() first)
.on_click(|ev: &ClickEvent, window, cx| { ... })

// With view state capture
let on_close = cx.listener(move |this: &mut AppRoot, _ev: &ClickEvent, _window, cx| {
    this.screen = Screen::Home(Default::default());
    cx.notify();
});
```

## Related Documentation

- [Project Structure](./project-structure.md) - File organization and directory layout
- [Data Storage](./database-schema.md) - JSON file formats and storage layout
- [Engine API](./api-routes.md) - Engine command function reference
- [GUI Architecture](./gui-architecture.md) - gpui components, screens, styling conventions