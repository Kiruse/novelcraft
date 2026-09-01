# Engine API

This document describes the engine command functions available in NovelCraft. Since this is an offline desktop app with no server, all API functionality is provided as plain Rust async functions in the `novelcraft-engine` library crate, called directly by the GUI crate.

## Overview

The engine crate (`novelcraft-engine`) exposes all functionality as `pub async fn` in `engine/src/commands/`. The GUI crate calls them via `novelcraft_engine::commands::*`. No IPC, no event bus, no code generation.

### Capabilities

- **LLM operations** — streaming proxy to OpenAI-compatible LLM APIs
- **File operations** — export/import sessions, native file/folder pickers
- **Data persistence** — filesystem-based JSON storage (no database)
- **Session management** — CRUD for vignette sessions, pages, and state snapshots
- **Profile management** — CRUD for player profiles
- **Story management** — read/write story definitions
- **Lore queries** — search lore entries by keyword
- **Game agent** — backend-driven agent loop for LLM gameplay sessions

### Calling from the GUI

```rust
use novelcraft_engine::AppState;
use novelcraft_engine::commands;

let sessions = commands::session::session_list(&app).await?;
commands::session::session_create(&app, id, None, "Title", None).await?;
commands::session::session_delete(&app, id).await?;
```

## Available Commands

### LLM Proxy

#### `prompt`

Streams an LLM response via callback closures. The engine calls the OpenAI-compatible chat completions API, parses SSE frames via `util::process_stream()`, and invokes callbacks for each event.

**File:** `engine/src/commands/llm.rs`

**Parameters:**

- `app: &AppState`
- `request: LlmPromptRequest`
- `on_text: impl Fn(String)` — text content chunk
- `on_reasoning: impl Fn(String)` — reasoning/thinking chunk
- `on_tool_call: impl Fn(LlmToolCallDelta)` — tool/function call streaming delta
- `on_error: impl Fn(String)` — error message
- `on_done: impl Fn(String, Option<LlmUsage>)` — stream complete (finish_reason, usage?)

**Returns:** `Result<()>`

**`LlmPromptRequest` fields:**

```rust
pub struct LlmPromptRequest {
    pub model: Option<String>,          // Usage ID (e.g., "storyteller", "suggestions")
    pub persona: Option<String>,        // System persona (e.g., PERSONA_PLATFORM)
    pub messages: Vec<LlmMessage>,      // Conversation history
    pub context: Option<serde_json::Value>,  // Optional additional context
    pub request_id: Option<String>,     // Optional request scoping
    pub tools: Option<Vec<LlmTool>>,    // Optional function/tool calling
}
```

**Model resolution:** The `model` field is a **usage ID** (e.g. `"storyteller"`, `"suggestions"`). The engine looks up the corresponding `ModelConfig` via `AppState.models.lock().await.get_config(usage)`. The actual LLM API model identifier (`config.model_id`) is sent in the API request body.

**`LlmMessage` fields:**

```rust
pub struct LlmMessage {
    pub author: String,            // "system" | "user" | "ai" | "tool"
    pub content: String,
    pub tool_call_id: Option<String>,
    pub tool_calls: Option<Vec<FunctionCall>>,
}
```

**`LlmToolCallDelta` fields:**

```rust
pub struct LlmToolCallDelta {
    pub index: u64,
    pub id: Option<String>,
    pub name: Option<String>,
    pub arguments_delta: String,
}
```

**`LlmDonePayload` fields:**

```rust
pub struct LlmUsage {
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub total_tokens: u64,
}
```

**Implementation types (Rust):** Types are split across three modules:

- **`engine/src/util.rs`** — SSE stream parsing: `StreamEvent` enum (`Text`, `Reasoning`, `ToolCall`, `Done`) and `process_stream()` async function.
- **`engine/src/infer/api.rs`** — OpenAI API wire types: `ChatCompletionRequest`, `StreamResponse`, `StreamChoice`, `StreamDelta`, etc.
- **`engine/src/infer/internal.rs`** — Command-level types: `ModelConfig`, `LlmMessage`, `LlmTool`, `LlmPromptRequest`, etc.

**Backward compatibility:** New fields use `#[serde(default)]` and are optional. Existing callers passing the old shape work unchanged.

#### `list_models`

Returns the configured model registry.

**File:** `engine/src/commands/llm.rs`

**Parameters:** `&AppState`

**Returns:** `Result<Models>`

```rust
pub struct Models {
    pub storyteller: ModelConfig,
    pub suggestions: ModelConfig,
}

pub struct ModelConfig {
    pub model_id: String,       // e.g., "zai-org/glm-4.6v-flash"
    pub base_url: String,       // e.g., "http://localhost:1234/v1"
    pub api_key: Option<String>,
}
```

#### `save_models`

Persists model configuration to disk (`{dataDir}/models.json`).

**File:** `engine/src/commands/llm.rs`

**Parameters:** `&AppState, Models`

**Returns:** `Result<()>`

#### `ping_hosts`

Checks liveness of all unique LLM host URLs. Sends GET to `{base_url}/models` with 5s timeout per host. Returns unreachable hosts.

**File:** `engine/src/commands/llm.rs`

**Parameters:** `&AppState`

**Returns:** `Result<Vec<UnreachableHost>>`

```rust
pub struct UnreachableHost {
    pub url: String,
    pub error: String,
}
```

#### `ping_host`

Checks liveness of a single LLM host URL.

**File:** `engine/src/commands/llm.rs`

**Parameters:** `url: String, api_key: Option<String>`

**Returns:** `Result<Option<String>>` — `None` if reachable, `Some(error)` if not.

### Game Agent

Game agent commands provide a backend-driven agent loop for LLM gameplay sessions.

**File:** `engine/src/commands/game.rs`

#### `game_prompt`

Starts an agent loop for a game session. Reads session history, adds prompt as user message (if non-empty), iterates LLM calls (up to `max_agent_steps` from `AppState.config`). Streams events via the `on_event` callback.

**Parameters:**

- `app: &AppState`
- `session_id: String`
- `prompt: String`
- `on_event: impl Fn(GamePromptEvent)` — callback for streaming events

**Returns:** `Result<GamePromptResult>`

```rust
pub struct GamePromptResult {
    pub stream_id: String,    // UUID for identification
}
```

**Event variants:**

| Variant | Fields | Description |
|---------|--------|-------------|
| `Reasoning` | `step: u8, delta: String` | Reasoning/thinking chunk |
| `Text` | `step: u8, delta: String` | Text content chunk |
| `ToolCalls` | `step: u8, text: Option<String>, calls: Vec<ToolCall>` | Tool calls received |
| `Done` | `finish_reason: String, usage: Option<LlmUsage>` | Agent loop completed |
| `Error` | `GamePromptError` | Error |

#### `game_fork`

Forks a game session at a given page index, then runs the agent loop.

**Parameters:**

- `app: &AppState`
- `session_id: String`
- `page_index: usize`
- `prompt: String`
- `on_event: impl Fn(GamePromptEvent)`

**Returns:** `Result<GamePromptResult>`

#### `game_sessions`

Lists all game sessions by scanning the sessions directory.

**Parameters:** `&AppState`

**Returns:** `Result<Vec<SessionV1>>`

#### `game_page`

Gets a specific page from a game session by index. Uses an LRU cache (capacity 32) keyed by `(session_id, batch_num)`.

**Parameters:** `&AppState, session_id: String, page: usize`

**Returns:** `Result<PageV1>`

### File Operations

#### `export_session`

Writes a session to a JSON file on disk.

**File:** `engine/src/commands/fs.rs`

**Parameters:** `&AppState, session_id: String, file_path: String, data: ExportData`

**Returns:** `Result<()>`

#### `import_session`

Reads a session from a JSON file on disk.

**File:** `engine/src/commands/fs.rs`

**Parameters:** `file_path: String`

**Returns:** `Result<ExportData>`

#### `pick_file`

Opens a native file picker dialog.

**File:** `engine/src/commands/fs.rs`

**Parameters:** `filters: Option<Vec<(String, Vec<String>)>>`

**Returns:** `Result<Option<String>>`

#### `pick_folder`

Opens a native folder picker dialog.

**File:** `engine/src/commands/fs.rs`

**Parameters:** None

**Returns:** `Result<Option<String>>`

### Session Management

Session commands manage vignette/gameplay sessions and their associated pages and state snapshots. All session data is stored as JSON files under `{dataDir}/sessions/{sessionUUID}/`.

**File:** `engine/src/commands/session.rs`

#### `session_list`

Lists all sessions.

**Parameters:** `&AppState`

**Returns:** `Result<Vec<SessionMeta>>`

```rust
pub struct SessionMeta {
    pub version: u8,
    pub id: String,
    pub story_id: Option<String>,
    pub title: String,
    pub description: Option<String>,
    pub disposition: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}
```

#### `session_create`

Creates a new session directory with `meta.json` and an initial head snapshot.

**Parameters:** `&AppState, sessionId: String, storyId: Option<String>, title: String, description: Option<String>`

**Returns:** `Result<()>`

#### `session_delete`

Deletes an entire session directory and all its files.

**Parameters:** `&AppState, sessionId: String`

**Returns:** `Result<()>`

#### `session_load`

Loads a full session: metadata, all pages, and the head snapshot.

**Parameters:** `&AppState, sessionId: String`

**Returns:** `Result<SessionLoadResult>`

```rust
pub struct SessionLoadResult {
    pub version: u8,
    pub meta: SessionMeta,
    pub pages: Vec<PageEntry>,
    pub head_snapshot: Option<Snapshot>,
}
```

#### `session_save_meta`

Updates session metadata.

**Parameters:** `&AppState, sessionId: String, meta: SessionMeta`

**Returns:** `Result<()>`

#### `session_push_page`

Appends a new page to the appropriate batch file.

**Parameters:** `&AppState, sessionId: String, page: PageEntry`

**Returns:** `Result<()>`

```rust
pub struct PageEntry {
    pub id: String,
    pub system: Option<String>,
    pub prompt: Option<String>,
    pub response: Option<String>,
    pub tool_calls: Option<String>,  // JSON array
    pub created_at: String,
}
```

#### `session_update_page`

Updates an existing page in place.

**Parameters:** `&AppState, sessionId: String, pageIndex: usize, page: PageEntry`

**Returns:** `Result<()>`

#### `session_truncate_pages`

Deletes all pages at and after the given page index.

**Parameters:** `&AppState, sessionId: String, pageIndex: usize`

**Returns:** `Result<()>`

#### `session_get_head_snapshot`

Returns the current head snapshot.

**Parameters:** `&AppState, sessionId: String`

**Returns:** `Result<Option<Snapshot>>`

```rust
pub struct Snapshot {
    pub version: u8,
    pub page_index: usize,
    pub data: serde_json::Value,  // { [moduleType]: moduleState }
}
```

#### `session_save_head_snapshot`

Writes or replaces the head snapshot file.

**Parameters:** `&AppState, sessionId: String, snapshot: Snapshot`

**Returns:** `Result<()>`

#### `session_delete_head_snapshot`

Deletes the head snapshot file.

**Parameters:** `&AppState, sessionId: String`

**Returns:** `Result<()>`

#### `session_find_snapshot_before`

Finds the youngest checkpoint snapshot with `page_index < pageIndex`.

**Parameters:** `&AppState, sessionId: String, pageIndex: usize`

**Returns:** `Result<Option<Snapshot>>`

#### `session_save_checkpoint`

Saves a checkpoint snapshot.

**Parameters:** `&AppState, sessionId: String, batch: usize, snapshot: Snapshot`

**Returns:** `Result<()>`

#### `session_delete_checkpoints_from`

Deletes all checkpoint snapshots at or after the given page index.

**Parameters:** `&AppState, sessionId: String, pageIndex: usize`

**Returns:** `Result<()>`

### Profile Management

Profile commands manage player profiles stored in `{dataDir}/profiles.json`. Profiles are held in memory via `OnceCell<Mutex<ProfilesFile>>`.

**File:** `engine/src/commands/profile.rs`

#### `profile_list`

Returns all profiles along with the active profile ID.

**Parameters:** `&AppState`

**Returns:** `Result<ProfileListResult>`

```rust
pub struct ProfileListResult {
    pub profiles: Vec<Profile>,
    pub active_id: Option<String>,
}
```

#### `profile_create`

Adds a new profile.

**Parameters:** `&AppState, id: String, name: String, fields: serde_json::Value, created_at: String`

**Returns:** `Result<()>`

#### `profile_update`

Updates an existing profile's name and fields.

**Parameters:** `&AppState, id: String, name: String, fields: serde_json::Value`

**Returns:** `Result<()>`

#### `profile_delete`

Removes a profile. Clears `active_id` if the deleted profile was active.

**Parameters:** `&AppState, id: String`

**Returns:** `Result<()>`

#### `profile_set_active`

Sets a profile as active.

**Parameters:** `&AppState, id: String`

**Returns:** `Result<()>`

### Story Management

Story commands manage story definitions stored as individual JSON files.

**File:** `engine/src/commands/story.rs`

#### `story_get`

Returns a story by ID.

**Parameters:** `&AppState, storyId: String`

**Returns:** `Result<StoryEntry>`

```rust
pub struct StoryEntry {
    pub version: u8,
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub config: serde_json::Value,
    pub created_at: String,
    pub updated_at: String,
}
```

#### `story_save`

Creates or replaces a story file.

**Parameters:** `&AppState, story: StoryEntry`

**Returns:** `Result<()>`

### Lore Queries

Lore commands search lore entries for a given story.

**File:** `engine/src/commands/lore.rs`

#### `lore_query`

Searches lore entries by title or content using case-insensitive substring matching.

**Parameters:** `&AppState, storyId: String, query: String`

**Returns:** `Result<LoreQueryResult>`

```rust
pub struct LoreQueryResult {
    pub results: Vec<LoreEntry>,
}
```

## Adding New Commands

1. Create a new `pub async fn` in the appropriate `engine/src/commands/*.rs` file
2. Take `&AppState` as the first parameter (or omit for stateless operations)
3. Return `Result<T>` using the engine's error type
4. Ensure parameter/return types derive `Serialize`/`Deserialize` as needed
5. Call from the GUI crate via `novelcraft_engine::commands::*`

```rust
// engine/src/commands/my_module.rs
use crate::{AppState, error::Result};

pub async fn my_command(app: &AppState, param: String) -> Result<String> {
    Ok(format!("Received: {}", param))
}
```

```rust
// gui/src/main.rs
use novelcraft_engine::commands::my_module;

let result = my_module::my_command(&app, "hello".to_string()).await?;
```

## Related Documentation

- [Code Conventions](./code-conventions.md) - Import patterns and async functions
- [Data Storage](./database-schema.md) - JSON file formats and storage layout
- [Project Structure](./project-structure.md) - File organization for commands
- [GUI Architecture](./gui-architecture.md) - gpui GUI components and screens