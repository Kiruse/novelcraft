# Project Structure

This document outlines the complete folder organization and directory layout for NovelCraft.

NovelCraft is a Rust/Cargo workspace split into two crates: **novelcraft-engine** (pure Rust library) and **novelcraft-gui** (GPUI-based binary). Licensed under GPL-3.0. Offline-first, single-user, no server, no database.

## Directory Tree

```
novelcraft/
├── justfile                          # Build orchestration (just commands)
├── Cargo.toml                        # Workspace root (members = ["engine", "gui"], resolver = "2")
├── rust-toolchain.toml               # Pins Rust 1.97.1
├── AGENTS.md                         # High-level overview for AI agents
├── README.md                         # Project overview, prerequisites, build instructions
├── engine/                           # Rust library crate (novelcraft-engine)
│   ├── Cargo.toml                    # Engine dependencies
│   └── src/
│       ├── lib.rs                    # Crate root — re-exports modules
│       ├── util.rs                   # SSE stream parsing (StreamEvent, process_stream), file I/O helpers
│       ├── config.rs                 # App configuration (NovelCraftConfig)
│       ├── error.rs                  # Error types
│       ├── commands/
│       │   ├── mod.rs                # Module barrel
│       │   ├── paths.rs              # Centralized data/config directory resolution
│       │   ├── llm.rs                # LLM proxy (HTTP streaming via reqwest, callback-based, model config)
│       │   ├── game.rs               # Game agent loop (game_prompt, game_fork, game_sessions, game_page)
│       │   ├── profile.rs            # Profile persistence (OnceCell<Mutex<ProfilesFile>>)
│       │   ├── session.rs            # Session/page/snapshot persistence (JSON files)
│       │   ├── story.rs              # Story persistence (JSON files)
│       │   ├── lore.rs               # Lore persistence (JSON files)
│       │   └── fs.rs                 # File operations (export/import, file dialogs)
│       ├── markdown/
│       │   ├── mod.rs                # Module barrel
│       │   └── todo.rs               # TodoItem/TodoList parsing & diffing
│       ├── game/                     # Game engine types
│       │   ├── mod.rs                # Module barrel
│       │   ├── engine.rs             # GameEngine (session loading, page CRUD, fork, history, LRU page cache)
│       │   ├── session.rs            # Game session model (SessionV1, file-based persistence)
│       │   ├── pages.rs              # Page types (PageV1, PageBatch, ResponseV1)
│       │   ├── state.rs              # AppState (config + profiles, initialized in lib.rs setup)
│       │   └── module.rs             # Game module abstractions
│       └── infer/                    # Inference types
│           ├── mod.rs                # Module barrel (pub mod api, pub mod internal)
│           ├── api.rs                # OpenAI API types (request/response structs for SSE)
│           └── internal.rs           # Command-level types (ModelConfig, LlmMessage, LlmTool, etc.)
├── gui/                              # Rust binary crate (novelcraft-gui, binary name: novelcraft)
│   ├── Cargo.toml                    # GUI dependencies (novelcraft-engine, gpui, gpui_platform, log)
│   └── src/
│       ├── main.rs                   # Entry point — gpui app initialization, AppRoot view
│       ├── util.rs                   # Loggable trait, LogLevel enum, Result<T,E> blanket impl
│       ├── theme.rs                  # Theme struct (bg, text colors)
│       ├── comp.rs                   # Reusable components (root(), SettingsGear)
│       └── screens/
│           ├── mod.rs                # Screen enum, Default impl
│           ├── home.rs               # Home screen render function
│           ├── settings.rs           # Settings screen render function
│           ├── gameplay.rs           # Gameplay screen (placeholder)
│           └── story.rs              # Story screen (placeholder)
└── docs/                             # Project documentation
    ├── project-structure.md          # This file
    ├── code-conventions.md           # Code styling and conventions
    ├── database-schema.md            # JSON file formats and storage layout
    ├── api-routes.md                 # Engine command function reference
    ├── gui-architecture.md           # gpui GUI components, screens, styling conventions
    └── frontend-architecture.md      # [vestigial] Vue/Tauri frontend (no longer used)
```

## Key Directories

### `engine/` — Rust Library Crate (`novelcraft-engine`)

Pure Rust library with all business logic. No UI framework coupling.

**`engine/src/`**
- `lib.rs` — Crate root, re-exports modules
- `util.rs` — SSE stream parsing (`StreamEvent` enum, `process_stream()`), file I/O helpers (`serialize`, `deserialize`, `ensure_dir`)
- `config.rs` — `NovelCraftConfig` struct (holds `max_agent_steps: u8`, default 10). Loaded from `{configDir}/config.json`. Held in `AppState.config: Mutex<NovelCraftConfig>`.
- `error.rs` — Error types using `thiserror`
- `commands/` — Engine command functions (plain `pub async fn`)
  - `paths.rs` — `data_dir()`, `config_dir()` — centralized path resolution via `dirs::data_dir()` / `dirs::config_dir()`
  - `llm.rs` — LLM proxy: builds requests, streams from OpenAI-compatible API via `reqwest`, delegates SSE parsing to `util::process_stream()`, invokes callback closures. Model registry (`Models` struct) in `AppState.models: Mutex<Models>`.
  - `game.rs` — Game agent loop: `game_prompt` runs an async agent loop reading session history, iterating LLM calls (up to `max_agent_steps`). `game_fork` truncates at a page index then runs the agent loop. Events delivered via generic `on_event: F` callback.
  - `profile.rs` — Profile CRUD for `profiles.json`. Uses `OnceCell<Mutex<ProfilesFile>>` pattern.
  - `session.rs` — Session/page/snapshot CRUD. All types include `version` field with `read_versioned_json()`.
  - `story.rs` — Story CRUD for `stories/{id}.json`. Version-gated deserialization.
  - `lore.rs` — Lore query for `lore/{id}.json`. Version-gated deserialization.
  - `fs.rs` — File operations: export/import session JSON, native file/folder picker dialogs.
- `game/` — Game engine types (separate from `commands/game.rs`)
  - `engine.rs` — `GameEngine`: loads game sessions, manages page batches, provides history for prompting, page CRUD, fork. Uses LRU cache (`moka::future::Cache`, capacity 8).
  - `session.rs` — `SessionV1`: game session model with file-based persistence.
  - `pages.rs` — Page types: `PageV1`, `PageBatch`, `ResponseV1`.
  - `state.rs` — `AppState`: holds `config`, `profiles`, and `models` mutexes.
  - `module.rs` — Game module abstractions.
- `infer/` — LLM API type definitions
  - `api.rs` — OpenAI API wire types: `ChatCompletionRequest`, `StreamResponse`, etc.
  - `internal.rs` — Command-level types: `ModelConfig`, `LlmMessage`, `LlmPromptRequest`, etc.
- `markdown/` — Markdown parsing utilities
  - `todo.rs` — `TodoItem`, `TodoList`, `TodoListDiff` — parsing & diffing with unit tests

### `gui/` — Rust Binary Crate (`novelcraft-gui`, binary name: `novelcraft`)

Native GUI using gpui (from the Zed editor repo). Depends on `novelcraft-engine` as a library.

**`gui/src/`**
- `main.rs` — Binary entry point, `AppRoot` view with screen dispatch
- `util.rs` — `Loggable` trait, `LogLevel` enum, blanket `impl Loggable for Result<T, E>`
- `theme.rs` — `Theme` struct (bg/text colors, `dark()` constructor)
- `comp.rs` — Reusable components: `root()` (themed div), `SettingsGear` (gpui View)
- `screens/` — Screen render functions (stateless, return `Div`)
  - `mod.rs` — `Screen` enum
  - `home.rs` — Home screen (title, settings gear)
  - `settings.rs` — Settings screen (header, close button)
  - `gameplay.rs` — Gameplay screen (placeholder)
  - `story.rs` — Story screen (placeholder)

**Note:** `gui/src/` also contains vestigial Vue 3 frontend files (`package.json`, `src/App.vue`, etc.) from the previous Tauri architecture. These are no longer used and should not be modified.

### `docs/` — Documentation

Comprehensive project documentation for developers and contributors.

## Build Orchestration

The root `justfile` is the single source of truth for all build commands.

| Recipe | Purpose |
|--------|----------|
| `just dev` | Run the app (`cargo run --bin novelcraft`) |
| `just build` | Build entire workspace (`cargo build`) |
| `just check` | Cargo check (entire workspace) |
| `just check-engine` | Cargo check (`novelcraft-engine` only) |
| `just check-gui` | Cargo check (`novelcraft-gui` only) |
| `just clippy` | Cargo clippy (entire workspace) |
| `just fmt` | Cargo fmt (entire workspace) |
| `just fmt-check` | Cargo fmt --check (entire workspace) |

## File Naming Conventions

### Rust Commands
- snake_case: `llm.rs`, `fs.rs`, `session.rs`, `story.rs`, `lore.rs`
- One file per domain in `engine/src/commands/`

### Rust Source Files
- snake_case: `main.rs`, `util.rs`, `config.rs`, `error.rs`

## Related Documentation

- [Code Conventions](./code-conventions.md) - Styling guidelines and import patterns
- [Data Storage](./database-schema.md) - JSON file formats and storage layout
- [Engine API](./api-routes.md) - Engine command function reference
- [GUI Architecture](./gui-architecture.md) - gpui screens, components, and styling conventions