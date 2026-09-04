# Agents Documentation

High-level overview for AI agents working on the NovelCraft project. For comprehensive documentation, see the [detailed docs](./docs/).

**IMPORTANT:** Whenever you make changes, assert validity by running `just check`, then summarize & document the changes with the `@docs-writer` subagent.

## Architecture Overview

NovelCraft is a **Rust/Cargo workspace desktop app** using gpui for rendering — fully offline-first, single-user, no server.

- **Engine** (`engine/`, crate `novelcraft-engine`) — pure Rust library crate with all business logic. Handles LLM proxy (HTTP streaming via `reqwest`, SSE parsing via `util.rs`), file operations (export/import, file dialogs), data persistence (sessions, profiles, stories, lore as JSON files), and game agent loop. No UI framework coupling.
- **GUI** (`gui/`, crate `novelcraft-gui`, binary `novelcraft`) — Rust binary crate using **gpui** and **gpui_platform** (from the Zed repo) for native rendering. Depends on `novelcraft-engine` as a library. Entry point is `gui/src/main.rs`.
- **Filesystem persistence** — all data (stories, sessions, pages, state snapshots, lore entries, profiles) stored as JSON files under the platform data directory (resolved via `dirs::data_dir()`). Path resolution is centralized in `engine/src/commands/paths.rs`.
- **No server, no auth, no database** — single-user desktop application with loose JSON files
- **LLM calls** go through engine functions — `commands::llm::prompt()` takes a request struct and streams responses via callback closures (`on_text`, `on_reasoning`, `on_tool_call`, `on_done`, `on_error`). The game agent loop (`commands::game`) uses a generic `on_event: F` callback.
- **Story sharing** is file-based — export/import JSON files via native file dialogs
- **Build orchestration** is via a root `justfile` — single Cargo workspace, no top-level `package.json`
- **AppState** uses `Arc<Mutex<T>>` pattern — constructed via `AppState::init()`, shared across engine functions by reference (`&AppState`). No framework-managed state.
- **Rust toolchain** pinned to 1.97.1 via `rust-toolchain.toml` (required by gpui)

## Quick Reference

### Project Structure

```
novelcraft/
├── justfile                    # Build orchestration (just commands)
├── Cargo.toml                  # Workspace root (members = ["engine", "gui"], resolver = "2")
├── rust-toolchain.toml         # Pins Rust 1.97.1
├── engine/                     # Rust library crate (novelcraft-engine)
│   ├── Cargo.toml              # Engine dependencies
│   └── src/
│       ├── lib.rs              # Crate root — re-exports modules
│       ├── util.rs             # SSE stream parsing (StreamEvent enum, process_stream()), file I/O helpers
│       ├── config.rs           # App configuration
│       ├── error.rs            # Error types
│       ├── commands/
│       │   ├── mod.rs          # Module barrel
│       │   ├── paths.rs        # Centralized data/config directory resolution (dirs::data_dir, dirs::config_dir)
│       │   ├── llm.rs          # LLM proxy (HTTP streaming via reqwest, callback-based events, model config)
│       │   ├── game.rs         # Game agent loop (game_prompt, game_fork, game_sessions, game_page)
│       │   ├── profile.rs      # Profile persistence (OnceCell<Mutex<ProfilesFile>>)
│       │   ├── session.rs      # Session/page/snapshot persistence (JSON files)
│       │   ├── story.rs        # Story persistence (JSON files)
│       │   ├── lore.rs         # Lore persistence (JSON files)
│       │   └── fs.rs           # File operations (export/import, file dialogs)
│       ├── markdown/           # Markdown parsing utilities
│       │   ├── mod.rs          # Module barrel (re-exports todo)
│       │   └── todo.rs         # TodoItem/TodoList parsing & diffing (with unit tests)
│       ├── game/               # Game engine types (GameEngine, SessionV1, etc.)
│       └── infer/              # Inference types
│           ├── mod.rs          # Module barrel (pub mod api, pub mod internal)
│           ├── api.rs          # OpenAI API types (request/response structs for SSE)
│           └── internal.rs     # Command-level types (ModelConfig, LlmMessage, LlmTool, LlmPromptRequest, etc.)
├── gui/                        # Rust binary crate (novelcraft-gui, binary name: novelcraft)
│   ├── Cargo.toml              # GUI dependencies (novelcraft-engine, gpui, gpui_platform, log)
│   └── src/
│       ├── main.rs             # Entry point — gpui app initialization
│       └── util.rs             # Loggable trait, LogLevel enum, Result<T,E> blanket impl
└── docs/                       # Comprehensive documentation
    ├── gui-architecture.md # gpui GUI components, screens, styling conventions
```

**Note:** The `gui/` directory also contains vestigial Vue 3 frontend files (`package.json`, `src/App.vue`, etc.) from the previous Tauri architecture. These are no longer used and should not be modified.

---

## Key Conventions Summary

### File Organization

| Category | Location | Pattern |
|----------|----------|---------|
| Engine Commands | `engine/src/commands/` | One file per domain (`llm.rs`, `game.rs`, `profile.rs`, `session.rs`, `story.rs`, `lore.rs`, `fs.rs`, `paths.rs`) |
| Engine Utilities | `engine/src/util.rs` | SSE stream parsing (`StreamEvent`, `process_stream`), file I/O helpers (`serialize`, `deserialize`, `ensure_dir`) |
| Markdown Utilities | `engine/src/markdown/` | `todo.rs` (`TodoItem`, `TodoList`, `TodoListDiff`, `TodoList::diff` — parsing & diffing with unit tests) |
| Engine Types | `engine/src/infer/` | `api.rs` (OpenAI API types), `internal.rs` (command-level types) |
| Game Engine | `engine/src/game/` | Game agent types (`GameEngine`, `SessionV1`) |
| GUI Entry | `gui/src/main.rs` | Binary entry point, gpui app setup |
| GUI Utilities | `gui/src/util.rs` | `Loggable` trait, `LogLevel` enum, blanket `impl Loggable for Result<T, E>` |
| Path Resolution | `engine/src/commands/paths.rs` | `data_dir()`, `config_dir()` — single source of truth for filesystem paths |

### Import Patterns

- **Within engine**: Standard Rust `mod`/`use` paths. No special aliases.
- **GUI depends on engine**: `use novelcraft_engine::commands::...` (or re-exports from `novelcraft_engine::*`).

### gpui Action Dispatch

Inside event handlers (`on_click`, etc.), always dispatch actions via `window.dispatch_action(Box<dyn Action>, cx)` — never `cx.dispatch_action` (fails with "window not found" during event handling, since the window is mid-update). `App::dispatch_action` is for app-level/global dispatch outside window updates (timers, menus). See [GUI Architecture](./docs/gui-architecture.md) for details.

### AI / Model Configuration

Models are configured in Rust, persisted to disk as JSON, and held in `AppState`.

- **Engine side**: `AppState` holds `models: Mutex<Models>`, initialized via `Models::load()` in `AppState::init()`. The `Models` struct (defined in `engine/src/commands/llm.rs`) has fields `storyteller: ModelConfig` and `suggestions: ModelConfig`, plus instance methods `get_config(usage)`, `all_configs()`, `load()`, `save()`.
- Each model entry maps a **usage role** (e.g. `"storyteller"`, `"suggestions"`) to `{ model_id, base_url, api_key? }`, where `model_id` is the actual LLM API model identifier (e.g. `"zai-org/glm-4.6v-flash"`)
- Default models point to `http://localhost:1234/v1` (local LLM server)

### Agent / LLM Integration

**All LLM calls go through engine functions.**

- **`commands::llm::prompt(app, request, callbacks)`** — takes `&AppState`, an `LlmPromptRequest`, and callback closures. Streams the response by calling callbacks as SSE frames arrive:
  - `on_text(chunk)` — text content chunk
  - `on_reasoning(chunk)` — reasoning/thinking chunk
  - `on_tool_call(delta)` — tool/function call streaming delta (`{ index, id?, name?, arguments_delta }`)
  - `on_error(message)` — error message
  - `on_done(finish_reason, usage?)` — stream complete
- Rust backend calls the OpenAI-compatible chat completions API, parses SSE frames via `util::process_stream()`, and invokes the appropriate callback. No event bus or pub/sub — direct callback invocation.

**Important terminology:** A "persona" is ONLY the system prompt passed as the `persona` parameter to the LLM call — it defines who the agent *is*. The sole persona used throughout the app is `PERSONA_PLATFORM`. Everything else — scene instructions (`SYSTEM_VIGNETTE_OPEN`), steering notes (`SYSTEM_STEER`), editor requests (`SYSTEM_INSTRUCT`), page-level `system` fields — are **NOT** personas. They are regular messages with `author: 'system'` injected into the conversation history to guide the agent's behavior.

### Data Persistence — Filesystem via Engine Commands

All structured data (stories, sessions, pages, state snapshots, lore entries, profiles) is stored as **JSON files** on disk, managed by engine command functions.

- **File layout** (under platform data directory, resolved by `commands::paths::data_dir()`):
  - `{dataDir}/sessions/{sessionUUID}/meta.json` — session metadata
  - `{dataDir}/sessions/{sessionUUID}/pages.{batch:03}.json` — pages batched in groups of 100
  - `{dataDir}/sessions/{sessionUUID}/state.head.json` — current head snapshot
  - `{dataDir}/sessions/{sessionUUID}/state.{batch:03}.json` — checkpoint snapshots
  - `{dataDir}/profiles.json` — all profiles
  - `{dataDir}/stories/{id}.json` — story definitions
  - `{dataDir}/lore/{id}.json` — lore entries
  - `{dataDir}/models.json` — LLM model configuration
- **Version-gated deserialization**: Every file format includes a `version` field. `read_versioned_json()` reads the version first, then dispatches to the correct deserializer (currently only v1). Future format changes add new match arms.
- **No transactions**: Each command call performs a single atomic file operation. Consumers drive sequential operations when multiple steps are needed.
- **Path resolution**: All file paths go through `engine/src/commands/paths.rs` which uses `dirs::data_dir()` and `dirs::config_dir()`. This replaced Tauri's `app.path()` API.

### Engine Command Functions

All command functions are plain `pub async fn` taking `&AppState` (or no state for read-only ops). No `#[tauri::command]` or `#[specta::specta]` decorators.

- **`engine/src/commands/session.rs`** — `session_list`, `session_create`, `session_delete`, `session_load`, `session_save_meta`, `session_push_page`, `session_update_page`, `session_truncate_pages`, `session_get_head_snapshot`, `session_save_head_snapshot`, `session_delete_head_snapshot`, `session_find_snapshot_before`, `session_save_checkpoint`, `session_delete_checkpoints_from`
- **`engine/src/commands/profile.rs`** — `profile_list`, `profile_create`, `profile_update`, `profile_delete`, `profile_set_active`. Uses `OnceCell<Mutex<ProfilesFile>>` pattern.
- **`engine/src/commands/story.rs`** — `story_get`, `story_save`
- **`engine/src/commands/lore.rs`** — `lore_query`
- **`engine/src/commands/llm.rs`** — `prompt` (streaming via callbacks), `list_models`, `save_models`, `ping_hosts`, `ping_host`
- **`engine/src/commands/game.rs`** — `game_prompt`, `game_fork`, `game_sessions`, `game_page` (agent loop with generic `on_event: F` callback)
- **`engine/src/commands/fs.rs`** — `export_session`, `import_session`, `pick_file`, `pick_folder`

```rust
use novelcraft_engine::AppState;
use novelcraft_engine::commands::{session, profile, story, lore};

let sessions = session::session_list(&app).await?;
let loaded = session::session_load(&app, id).await?;
session::session_save_meta(&app, id, updated_meta).await?;
session::session_push_page(&app, id, page_entry).await?;
session::session_delete(&app, id).await?;

let result = profile::profile_list(&app).await?;
profile::profile_create(&app, new_profile).await?;
profile::profile_set_active(&app, profile_id).await?;

let story = story::story_get(&app, id).await?;
story::story_save(&app, story_entry).await?;

let results = lore::lore_query(&app, id, "search").await?;
```

### Snapshot Lifecycle

State snapshots enable undo/fork by capturing module state at each page:

- **Session creation** — head snapshot (`state.head.json`) with empty state `{}`
- **After each page's tool calls** — head snapshot updated in place via `session::session_save_head_snapshot()`
- **Every 100 pages** — copy current head as checkpoint (`state.{batch}.json`) via `session::session_save_checkpoint()` before updating
- **Fork** — `GameEngine::fork(page_index)` truncates page batch files after the batch containing `page_index`, then reloads the session via `SessionV1::load` to obtain correct `tail_batches`, `page_count`, `batch_count`, and `gamestate`, and rebuilds the agent loop
- **State is immutable** — the canonical state is the snapshot data

### Game Agent Loop

The game agent loop (`commands::game`) iterates LLM calls until a final text response is received or tool calls are resolved.

- **`game_prompt(app, session_id, prompt, on_event)`** — reads session history, adds prompt as user message, iterates LLM calls (up to `max_agent_steps` from `AppState.config`)
- **`game_fork(app, session_id, page_index, prompt, on_event)`** — forks at page, then runs agent loop
- Event callback receives variants: `Reasoning { step, delta }`, `Text { step, delta }`, `ToolCalls { step, text?, calls }`, `Done { finish_reason, usage? }`, `Error(...)`

---

## Common Tasks

### Running the Application

All commands are managed by the root `justfile`. Run `just` with no arguments to list available recipes.

```bash
# Run the app (cargo run --bin novelcraft)
just dev

# Build
just build

# Check (cargo check)
just check

# Per-crate checking
just check-engine
just check-gui
```

### Type Checking & Linting

```bash
# Cargo check (entire workspace)
just check

# Cargo clippy
just clippy

# Cargo fmt / fmt --check
just fmt
just fmt-check
```

---

## Data Access Patterns

### All Data via Engine Functions

All data access goes through engine command functions — never direct file I/O from the GUI layer.

```rust
use novelcraft_engine::AppState;
use novelcraft_engine::commands::{session, profile, story, lore};

let sessions = session::session_list(&app).await?;
await session::session_create(&app, id, story_id, title, description)?;
let loaded = session::session_load(&app, id).await?;
session::session_save_meta(&app, id, updated_meta).await?;
session::session_push_page(&app, id, page_entry).await?;
session::session_update_page(&app, id, 0, updated_page).await?;
session::session_truncate_pages(&app, id, 5).await?;
session::session_delete(&app, id).await?;

let head = session::session_get_head_snapshot(&app, id).await?;
session::session_save_head_snapshot(&app, id, snap).await?;
session::session_delete_head_snapshot(&app, id).await?;
let snap = session::session_find_snapshot_before(&app, id, 5).await?;
session::session_save_checkpoint(&app, id, 0, snap).await?;
session::session_delete_checkpoints_from(&app, id, 5).await?;

let result = profile::profile_list(&app).await?;
profile::profile_create(&app, new_profile).await?;
profile::profile_update(&app, profile_id, updated_profile).await?;
profile::profile_delete(&app, profile_id).await?;
profile::profile_set_active(&app, profile_id).await?;

let story = story::story_get(&app, id).await?;
story::story_save(&app, story_entry).await?;

let results = lore::lore_query(&app, id, "search").await?;
```

**No transactions**: Each function call is independent. Consumers drive sequential operations.

---

## Engine API Reference

### LLM Proxy

| Function | Parameters | Returns | Description |
|----------|-----------|---------|-------------|
| `prompt` | `&AppState, LlmPromptRequest, callbacks` | `Result<()>` | Stream LLM response via callbacks |
| `list_models` | `&AppState` | `Result<Models>` | Get configured models |
| `save_models` | `&AppState, Models` | `Result<()>` | Save model configuration |
| `ping_hosts` | `&AppState` | `Result<Vec<UnreachableHost>>` | Check all LLM host liveness |
| `ping_host` | `url, api_key?` | `Result<Option<String>>` | Check single host liveness |

### File Operations

| Function | Parameters | Returns | Description |
|----------|-----------|---------|-------------|
| `export_session` | `&AppState, session_id, file_path, ExportData` | `Result<()>` | Write session to file |
| `import_session` | `file_path` | `Result<ExportData>` | Read session from file |
| `pick_file` | `filters?` | `Result<Option<String>>` | Open native file picker |
| `pick_folder` | — | `Result<Option<String>>` | Open native folder picker |

### Game Agent

| Function | Parameters | Returns | Description |
|----------|-----------|---------|-------------|
| `game_prompt` | `&AppState, session_id, prompt, on_event` | `Result<GamePromptResult>` | Start agent loop, streams via callback |
| `game_fork` | `&AppState, session_id, page_index, prompt, on_event` | `Result<GamePromptResult>` | Fork at page, then run agent loop |
| `game_sessions` | `&AppState` | `Result<Vec<SessionV1>>` | List all game sessions |
| `game_page` | `&AppState, session_id, page` | `Result<PageV1>` | Get a specific page from a game session |

### Sessions

| Function | Parameters | Returns | Description |
|----------|-----------|---------|-------------|
| `session_list` | `&AppState` | `Result<Vec<SessionMeta>>` | List all sessions |
| `session_create` | `&AppState, sessionId, storyId?, title, description?` | `Result<()>` | Create session with initial snapshot |
| `session_delete` | `&AppState, sessionId` | `Result<()>` | Delete session directory |
| `session_load` | `&AppState, sessionId` | `Result<SessionLoadResult>` | Load full session |
| `session_save_meta` | `&AppState, sessionId, meta` | `Result<()>` | Update session metadata |
| `session_push_page` | `&AppState, sessionId, page` | `Result<()>` | Append a new page |
| `session_update_page` | `&AppState, sessionId, pageIndex, page` | `Result<()>` | Update page in place |
| `session_truncate_pages` | `&AppState, sessionId, pageIndex` | `Result<()>` | Delete pages at/after index |
| `session_get_head_snapshot` | `&AppState, sessionId` | `Result<Option<Snapshot>>` | Get current head snapshot |
| `session_save_head_snapshot` | `&AppState, sessionId, snapshot` | `Result<()>` | Write head snapshot |
| `session_delete_head_snapshot` | `&AppState, sessionId` | `Result<()>` | Delete head snapshot |
| `session_find_snapshot_before` | `&AppState, sessionId, pageIndex` | `Result<Option<Snapshot>>` | Find youngest checkpoint before index |
| `session_save_checkpoint` | `&AppState, sessionId, batch, snapshot` | `Result<()>` | Save checkpoint snapshot |
| `session_delete_checkpoints_from` | `&AppState, sessionId, pageIndex` | `Result<()>` | Delete checkpoints at/after index |

### Profiles

| Function | Parameters | Returns | Description |
|----------|-----------|---------|-------------|
| `profile_list` | `&AppState` | `Result<ProfileListResult>` | List all profiles with active ID |
| `profile_create` | `&AppState, id, name, fields, created_at` | `Result<()>` | Create a profile |
| `profile_update` | `&AppState, id, name, fields` | `Result<()>` | Update a profile's name and fields |
| `profile_delete` | `&AppState, id` | `Result<()>` | Delete a profile |
| `profile_set_active` | `&AppState, id` | `Result<()>` | Set profile as active |

### Stories

| Function | Parameters | Returns | Description |
|----------|-----------|---------|-------------|
| `story_get` | `&AppState, storyId` | `Result<StoryEntry>` | Get story by ID |
| `story_save` | `&AppState, story` | `Result<()>` | Create or replace story |

### Lore

| Function | Parameters | Returns | Description |
|----------|-----------|---------|-------------|
| `lore_query` | `&AppState, storyId, query` | `Result<LoreQueryResult>` | Search lore entries |

---

## Agent Guidelines

### When to Add an Engine Command

- GUI needs access to a system capability (file system, native dialogs, etc.)
- New data persistence operations (new entity types, new query patterns)
- New operations on existing entities

**Process:**
1. Create a new `pub async fn` in the appropriate `engine/src/commands/*.rs` file
2. Take `&AppState` as the first parameter (or omit for stateless operations)
3. Return `Result<T>` using the engine's error type
4. Ensure parameter/return types derive `Serialize`/`Deserialize` as needed
5. Call from the GUI crate via `novelcraft_engine::commands::*`

### When to Modify Data Formats

- Entity data structures change (new fields, renamed fields)
- New entity types need to be persisted
- Page or snapshot formats need updating

**Process:**
1. Add/modify Rust structs in `engine/src/commands/session.rs`, `engine/src/commands/story.rs`, or `engine/src/commands/lore.rs`
2. Add a new version arm in `read_versioned_json()` if the format is backward-incompatible
3. Old-format files continue to work via version-gated deserialization

### When to Add a GUI Component

- Reusable UI patterns (gpui views/elements)
- Complex rendering that should be isolated into its own view struct

**Process:**
1. Create a new struct implementing `Render` in `gui/src/`
2. Use gpui's element system (`div()`, `child()`, etc.)
3. Register with the gpui context as needed

### When to Add a Game Module

- New gameplay mechanics that require LLM tool access
- New state that needs snapshot/fork support

**Process:**
1. Define the module with its tools and `init()` default state
2. Register in the game engine's module registry
3. Ensure state is serializable for snapshot persistence

---

## Justfile Commands

All build and development commands are in the root `justfile`.

| Recipe | Purpose |
|--------|---------|
| `just dev` | Run the app (`cargo run --bin novelcraft`) |
| `just build` | Build entire workspace (`cargo build`) |
| `just check` | Cargo check (entire workspace) |
| `just check-engine` | Cargo check (`novelcraft-engine` only) |
| `just check-gui` | Cargo check (`novelcraft-gui` only) |
| `just clippy` | Cargo clippy (entire workspace) |
| `just fmt` | Cargo fmt (entire workspace) |
| `just fmt-check` | Cargo fmt --check (entire workspace) |

## Key Dependencies

### Engine (engine/Cargo.toml, crate: novelcraft-engine)

| Crate | Purpose |
|-------|---------|
| `reqwest` (stream, json) | HTTP client for LLM proxy (streaming SSE) |
| `bytes` | Byte buffer utilities (SSE stream parsing) |
| `serde` / `serde_json` | Serialization |
| `tokio` (full) | Async runtime |
| `futures` | Async stream utilities |
| `dirs` | Platform directory resolution (`data_dir`, `config_dir`) |
| `uuid` (v4) | UUID generation |
| `chrono` | Timestamp handling |
| `thiserror` | Error derive macro |
| `log` / `env_logger` | Logging |
| `kiruklaw-agent-loop` | Agent loop utilities (git dependency) |

### GUI (gui/Cargo.toml, crate: novelcraft-gui, binary: novelcraft)

| Crate | Purpose |
|-------|---------|
| `novelcraft-engine` (path) | Business logic library |
| `gpui` (git, Zed main) | UI framework — views, elements, styling |
| `gpui_platform` (git, Zed main, features: font-kit, wayland, x11) | Platform integration — window management, app lifecycle |
| `log` | Logging facade (used by `util.rs` `Loggable` trait) |
