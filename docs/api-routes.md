# API Routes

This document describes the Tauri commands available in Novelcraft. Since this is a desktop app with no server, all "API" functionality is exposed as Tauri commands invoked from the frontend webview via tauri-specta generated bindings.

## Overview

The application uses Tauri commands as its IPC layer. Commands are defined in `engine/src/src/commands/` and registered in `engine/src/src/lib.rs`. The frontend calls them through **tauri-specta generated bindings** (`gui/src/bindings.ts`) with `commands.xxx()` syntax. The `unwrap()` helper in `gui/src/utils/index.ts` converts the `typedError` discriminated union back to throw-on-error behavior. The only raw `invoke()` remaining is `invoke('prompt', ...)` in `tauriLanguageModel.ts` (fire-and-forget for LLM streaming).

### Architecture

There is no HTTP server. All communication between the Vue frontend and Rust backend goes through Tauri's IPC:

- **LLM operations** — streaming proxy to OpenAI-compatible LLM APIs
- **File operations** — export/import sessions, native file/folder pickers
- **Data persistence** — filesystem-based JSON storage via Tauri commands (no database)
- **Session management** — CRUD for vignette sessions, pages, and state snapshots
- **Profile management** — CRUD for player profiles (in `engine/src/src/commands/profile.rs`)
- **Story management** — read/write story definitions
- **Lore queries** — search lore entries by keyword

### Plugin Permissions

Tauri v2 plugin permissions are configured in `engine/capabilities/default.json`. This capability grants the main window access to the `store`, `dialog`, and `fs` plugins. When adding a new Tauri plugin, its permissions must be added here.

### Calling Commands from Frontend

```typescript
import { commands } from '~/bindings';
import { unwrap } from '~/utils';

// Simple command
const models = await unwrap(commands.listModels());

// Command with parameters
await unwrap(commands.saveModels(updatedModels));

// LLM streaming (use the composable — never call invoke('prompt', ...) directly)
// import { streamLlmFull } from '~/composables/useLlmStream';
```

### tauri-specta Integration

All Tauri commands have `#[specta::specta]` annotations and all parameter/return structs derive `specta::Type`. Bindings are auto-generated at app startup (debug builds only) to `gui/src/bindings.ts`.

**BigInt fix:** `serde_json::Value` fields in structs use `#[specta(type = Any)]` attribute to export as TS `any`. Command parameters that are `serde_json::Value` directly use a custom `JsonAny` wrapper type (defined in `story.rs` and `lore.rs`).

## Available Commands

### LLM Proxy

#### `prompt`

Streams an LLM response via Tauri events. The Rust backend calls the OpenAI-compatible chat completions API, delegates SSE frame parsing to `util::process_stream()`, and emits events back to the frontend.

**File:** `engine/src/src/commands/llm.rs`

**Parameters:**

```typescript
{
  request: {
    model?: string;        // Usage ID (e.g., 'storyteller', 'suggestions')
    persona?: string;      // Optional system persona (e.g., PERSONA_PLATFORM)
    messages: Array<{
      author: string;      // 'system' | 'user' | 'ai' | 'tool'
      content: string;
      tool_call_id?: string;   // For tool result messages
      tool_calls?: Array<{    // For assistant messages with tool calls
        id: string;
        type: string;
        function: { name: string; arguments: string };
      }>;
    }>;
    context?: Record<string, unknown>;  // Optional additional context
    request_id?: string;    // Optional — scopes events to llm:{event}:{request_id}
    tools?: Array<{         // Optional — function/tool calling support
      name: string;
      description?: string;
      parameters?: unknown; // JSON Schema
    }>;
  }
}
```

**Returns:** `void` (emits events; does not resolve until after `llm:done` is emitted)

**Model resolution:** The `model` field in the request is a **usage ID** (e.g. `"storyteller"`, `"suggestions"`). The Rust backend looks up the corresponding `ModelConfig` via `AppState.models.lock().await.get_config(usage)` (the `Models` struct is held in `AppState`). The actual LLM API model identifier (`config.model_id`, e.g. `"zai-org/glm-4.6v-flash"`) is then sent as the `"model"` field in the API request body.

**Events emitted:**

All events use the base name unless `request_id` is provided, in which case they are scoped as `llm:{event}:{request_id}`.

| Event | Payload | Description |
|-------|---------|-------------|
| `llm:text` | `string` | Text content chunk from the LLM |
| `llm:reasoning` | `string` | Reasoning/thinking chunk (if supported) |
| `llm:tool_call` | `LlmToolCallDelta` | Tool/function call streaming delta |
| `llm:error` | `string` | Error message |
| `llm:done` | `LlmDonePayload` | Stream completed |

**Event payload types:**

```typescript
interface LlmToolCallDelta {
  index: number;
  id?: string;
  name?: string;
  arguments_delta: string;
}

interface LlmDonePayload {
  finish_reason: string;   // "stop" | "length" | "tool_calls" | "content_filter" | "error"
  usage?: {
    prompt_tokens: number;
    completion_tokens: number;
    total_tokens: number;
  };
}
```

**Implementation types (Rust):** The `prompt` command uses typed serde structs instead of `serde_json::Value` for all request/response handling. Types are split across three modules:

**`engine/src/util/mod.rs`** — SSE stream parsing (extracted from `llm.rs`):
- `StreamEvent` — enum with variants: `Text(String)`, `Reasoning(String)`, `ToolCall { index, id, name, arguments_delta }`, `Done { finish_reason, usage }`
- `process_stream()` — async function that takes a byte stream and a callback `Fn(StreamEvent)`, parses SSE frames, and invokes the callback for each event. Tracks `finish_reason` and `usage` state internally and emits `Done` when `[DONE]` is received or the stream ends. Uses `futures::StreamExt` for byte stream iteration.

**`engine/src/infer/api.rs`** — OpenAI API wire types (imported by `llm.rs` via `use crate::infer::api::*`):
- `FunctionCall` / `ToolCall` — typed tool call structs matching the OpenAI format (`{ id, type, function: { name, arguments } }`), used in `LlmMessage::tool_calls`
- `ApiChatMessage`, `ApiToolFunction`, `ApiTool`, `StreamOptions`, `ChatCompletionRequest` — typed structs for building the API request body (replaces `serde_json::json!()` macro calls)
- `StreamResponse`, `StreamUsage`, `StreamChoice`, `StreamDelta`, `StreamToolCall`, `StreamFunctionDelta` — typed structs for SSE response parsing (replaces manual `.get()` chaining on `serde_json::Value`)

**`engine/src/infer/internal.rs`** — Command-level types (extracted from `llm.rs`):
- `ModelConfig`, `LlmMessage`, `LlmTool`, `LlmToolCallDelta`, `LlmDonePayload`, `LlmUsage`, `LlmPromptRequest`, `UnreachableHost`, `PingHostRequest`
- All structs derive `specta::Type`. `serde_json::Value` fields use `#[specta(type = Any)]` to export as TS `any` (avoids BigInt issues).

Fields still using `serde_json::Value`: `LlmTool::parameters` (JSON Schema passthrough to API) and `LlmPromptRequest::context` (dead code, kept for frontend compatibility). Both have `#[specta(type = Any)]`.

**`engine/src/commands/llm.rs`** — command implementation. The `prompt` function builds the HTTP request, then calls `process_stream(response.bytes_stream(), &|event| match event { ... })` with a closure that maps `StreamEvent` variants to Tauri `emit_event` calls. SSE buffer/frame parsing logic was extracted to `util/mod.rs`, making `llm.rs` focused on request building and error handling.

**Backward compatibility:** All new fields (`persona` made optional, `request_id`, `tools`, `tool_call_id`, `tool_calls` on messages) use `#[serde(default)]` and are optional. Existing callers passing the old shape work unchanged. Events without `request_id` retain the same names as before.

**Frontend usage:** Always use `streamLlmFull()` or `streamLlm()` from `gui/src/composables/useLlmStream.ts`. The composable calls `invoke('prompt', ...)` without awaiting (fire-and-forget) because the Rust command resolves only after `llm:done` has been emitted. Never listen to Tauri events directly.

For AI SDK integration (e.g., `ConversationalArchetype` from `@stegakir/aikit`), use `createTauriModel(modelId)` from `gui/src/utils/tauriLanguageModel.ts` — this implements `LanguageModelV3` and handles scoped event routing automatically.

#### `list_models`

Returns the configured model registry.

**File:** `engine/src/src/commands/llm.rs`

**Parameters:** None

**Returns:** `Models`

```typescript
interface Models {
  storyteller: ModelConfig;
  suggestions: ModelConfig;
}

interface ModelConfig {
  model_id: string;       // Actual LLM API model identifier (e.g., 'zai-org/glm-4.6v-flash')
  base_url: string;       // API base URL (e.g., 'http://localhost:1234/v1')
  api_key?: string;       // Optional API key
}
```

#### `save_models`

Persists model configuration to disk (`{app_data_dir}/models.json`).

**File:** `engine/src/src/commands/llm.rs`

**Parameters:**

```typescript
{
  models: Models  // Models struct (same shape as list_models return)
}
```

**Returns:** `void`

#### `ping_hosts`

Checks liveness of all unique LLM host URLs configured in the model registry. Extracts `base_url` values from all model configs, sends a GET request to `{base_url}/models` with a 5-second timeout per host, and returns a list of hosts that did not respond with a 2xx status code. If a model config includes an `api_key`, it is sent as a `Bearer` token in the `Authorization` header.

**File:** `engine/src/src/commands/llm.rs`

**Parameters:** None

**Returns:** `Vec<UnreachableHost>`

```typescript
interface UnreachableHost {
  url: string;    // The base_url that failed to respond
  error: string;  // Error description (timeout, connection refused, non-2xx status, etc.)
}
```

**Implementation notes:**

- Model configs are extracted via `AppState.models.lock().await.all_configs()`, then the `MutexGuard` is dropped before async HTTP operations.
- Hosts are deduplicated — if multiple model configs share the same `base_url`, only one probe is sent.
- A host that responds with any 2xx status is considered reachable and omitted from the result.

#### `ping_host`

Checks liveness of a single LLM host URL. Sends a GET request to `{url}/models` with a 5-second timeout. Uses the same probe logic as `ping_hosts` but for a single URL instead of scanning the entire model registry.

**File:** `engine/src/src/commands/llm.rs`

**Parameters:**

```typescript
{
  request: {
    url: string;       // Base URL to probe (e.g., 'http://localhost:1234/v1')
    api_key?: string;  // Optional API key sent as Bearer token
  }
}
```

**Returns:** `Option<string>` — `null` if the host is reachable (2xx response), or an error string describing the failure (timeout, connection refused, non-2xx status, etc.).

**Frontend usage:** Called from the settings page with a 600ms debounce when the `base_url` field is edited, to provide immediate feedback on whether the entered URL is reachable.

### Game Agent

Game agent commands provide a backend-driven agent loop for LLM gameplay sessions. The agent reads session history, sends it to the LLM, and iterates until a final text response is produced or tool calls are resolved.

**File:** `engine/src/src/commands/game.rs`

The game agent uses `GameEngine` (from `engine/src/game/engine.rs`) to load session history and `AppState.config.max_agent_steps` to limit iterations.

#### `game_prompt`

Starts an agent loop for a game session. Spawns an async task that reads the session history, adds the prompt as a user message (if non-empty), and iterates LLM calls (up to `max_agent_steps` from `AppState.config`) until a final text response is received or tool calls are resolved.

**Parameters:**

```typescript
{
  session_id: string;   // Game session ID
  prompt: string;       // User prompt (trimmed; if empty, no user message is added)
}
```

**Returns:** `GamePromptResult`

```typescript
interface GamePromptResult {
  stream_id: string;    // UUID for scoping events
}
```

**Events emitted:**

Events use the `gamePrompt[{stream_id}]` pattern for scoping.

| Event Variant | Fields | Description |
|---------------|--------|-------------|
| `Reasoning` | `{ step: u8, delta: string }` | Reasoning/thinking chunk |
| `Text` | `{ step: u8, delta: string }` | Text content chunk |
| `ToolCalls` | `{ step: u8, text?: string, calls: ToolCall[] }` | Tool calls received (text is optional trailing text) |
| `Done` | `{ finish_reason: string, usage?: LlmUsage }` | Agent loop completed |
| `Error` | `GamePromptError` | Error (request failure or internal error) |

```typescript
interface GamePromptError {
  domain: 'request' | 'internal';
  // domain = 'request':
  status?: number;
  body?: string;
  // domain = 'internal':
  message?: string;
}
```

#### `game_fork`

Forks a game session at a given page index, then runs the same agent loop as `game_prompt`. Truncates all pages at and after `page_index`, then spawns the agent loop.

**Parameters:**

```typescript
{
  session_id: string;   // Game session ID
  page_index: number;   // Page index to fork at (0-based)
  prompt: string;       // User prompt (same semantics as game_prompt)
}
```

**Returns:** `GamePromptResult`

Uses the same event pattern as `game_prompt`.

#### `game_sessions`

Lists all game sessions by scanning the sessions directory.

**Parameters:** None

**Returns:** `Vec<SessionV1>`

#### `game_page`

Gets a specific page from a game session by index.

**Parameters:**

```typescript
{
  session_id: string;   // Game session ID
  page: number;         // Page index (0-based)
}
```

**Returns:** `PageV1`

### File Operations

#### `export_session`

Writes a session to a JSON file on disk.

**File:** `engine/src/src/commands/fs.rs`

**Parameters:**

```typescript
{
  session_id: string;
  file_path: string;
  data: ExportData;
}
```

**Returns:** `void`

#### `import_session`

Reads a session from a JSON file on disk.

**File:** `engine/src/src/commands/fs.rs`

**Parameters:**

```typescript
{
  file_path: string;
}
```

**Returns:** `ExportData`

#### `pick_file`

Opens a native file picker dialog.

**File:** `engine/src/src/commands/fs.rs`

**Parameters:**

```typescript
{
  filters?: Array<{ name: string; extensions: string[] }>;
}
```

**Returns:** `string | null` (selected file path, or null if cancelled)

#### `pick_folder`

Opens a native folder picker dialog.

**File:** `engine/src/src/commands/fs.rs`

**Parameters:** None

**Returns:** `string | null` (selected folder path, or null if cancelled)

### Session Management

Session commands manage vignette/gameplay sessions and their associated pages and state snapshots. All session data is stored as JSON files under `{appData}/sessions/{sessionUUID}/`.

**File:** `engine/src/src/commands/session.rs`

#### `session_list`

Lists all sessions.

**Parameters:** None

**Returns:** `SessionMeta[]`

```typescript
interface SessionMeta {
  version: 1;
  id: string;
  story_id?: string | null; // Associated story ID (absent for impromptu/freeform sessions)
  title: string;
  description?: string;
  disposition?: string;
  created_at: string;
  updated_at: string;
}
```

#### `session_create`

Creates a new session directory with `meta.json` and an initial head snapshot (`state.head.json` with empty state `{}`).

**Parameters:**

```typescript
{
  sessionId: string;
  storyId?: string | null; // Story ID (null for impromptu/freeform sessions)
  title: string;
  description?: string;
}
```

**Returns:** `void`

#### `session_delete`

Deletes an entire session directory and all its files (meta, pages, snapshots).

**Parameters:**

```typescript
{
  sessionId: string;
}
```

**Returns:** `void`

#### `session_load`

Loads a full session: metadata, all pages (all batches), and the head snapshot.

**Parameters:**

```typescript
{
  sessionId: string;
}
```

**Returns:** `SessionLoadResult`

```typescript
interface SessionLoadResult {
  version: 1;
  meta: SessionMeta;
  pages: PageEntry[];
  headSnapshot: Snapshot | null;
}
```

#### `session_save_meta`

Updates session metadata (`meta.json`).

**Parameters:**

```typescript
{
  sessionId: string;
  meta: SessionMeta;
}
```

**Returns:** `void`

#### `session_push_page`

Appends a new page to the appropriate batch file. Creates a new batch file if the current batch is full (100 pages per batch).

**Parameters:**

```typescript
{
  sessionId: string;
  page: PageEntry;
}
```

**Returns:** `void`

```typescript
interface PageEntry {
  id: string;
  system?: string;
  prompt?: string;
  response?: string;
  tool_calls?: string;  // JSON array of ToolCallRecord objects
  created_at: string;
}
```

#### `session_update_page`

Updates an existing page in place within its batch file.

**Parameters:**

```typescript
{
  sessionId: string;
  pageIndex: number;
  page: PageEntry;
}
```

**Returns:** `void`

#### `session_truncate_pages`

Deletes all pages at and after the given page index. Removes affected batch files entirely if the index falls on a batch boundary, otherwise truncates within the batch.

**Parameters:**

```typescript
{
  sessionId: string;
  pageIndex: number;
}
```

**Returns:** `void`

#### `session_get_head_snapshot`

Returns the current head snapshot (`state.head.json`).

**Parameters:**

```typescript
{
  sessionId: string;
}
```

**Returns:** `Snapshot | null`

```typescript
interface Snapshot {
  version: 1;
  page_index: number;
  data: Record<string, unknown>;  // { [moduleType]: moduleState }
}
```

Note: The generated binding type is `Snapshot`. The `useVignette` composable imports it as `Snapshot as SnapshotEntry` internally to avoid name collision with a local `Snapshot` interface.

#### `session_save_head_snapshot`

Writes or replaces the head snapshot file (`state.head.json`).

**Parameters:**

```typescript
{
  sessionId: string;
  snapshot: Snapshot;
}
```

**Returns:** `void`

#### `session_delete_head_snapshot`

Deletes the head snapshot file.

**Parameters:**

```typescript
{
  sessionId: string;
}
```

**Returns:** `void`

#### `session_find_snapshot_before`

Finds the youngest checkpoint snapshot with `page_index < pageIndex`. Used during fork to find the starting point for replay.

**Parameters:**

```typescript
{
  sessionId: string;
  pageIndex: number;
}
```

**Returns:** `Snapshot | null`

#### `session_save_checkpoint`

Saves a checkpoint snapshot to `state.{batch}.json`.

**Parameters:**

```typescript
{
  sessionId: string;
  batch: number;
  snapshot: Snapshot;
}
```

**Returns:** `void`

#### `session_delete_checkpoints_from`

Deletes all checkpoint snapshot files with `page_index >= pageIndex`. Used during fork to clean up orphaned checkpoints.

**Parameters:**

```typescript
{
  sessionId: string;
  pageIndex: number;
}
```

**Returns:** `void`

### Profile Management

Profile commands manage player profiles stored in `{appData}/profiles.json`. Profiles are held in memory via `OnceCell<Mutex<ProfilesFile>>` in `engine/src/src/commands/profile.rs`, initialized by `init_profiles()` called in `lib.rs` setup.

**File:** `engine/src/src/commands/profile.rs`

#### `profile_list`

Returns all profiles along with the active profile ID.

**Parameters:** None

**Returns:** `ProfileListResult`

```typescript
interface ProfileListResult {
  profiles: Profile[];
  active_id: string | null;  // ID of the active profile, or null if none set
}

interface Profile {
  id: string;           // UUID profile identifier
  name: string;         // Display name
  fields: any;          // Key-value fields (serde_json::Value)
  created_at: string;   // ISO timestamp
  updated_at: string;   // ISO timestamp
}
```

#### `profile_create`

Adds a new profile to `profiles.json`.

**Parameters:**

```typescript
{
  id: string;           // UUID
  name: string;         // Display name
  fields: any;          // Key-value fields (serde_json::Value)
  created_at: string;   // ISO timestamp (also used for updated_at)
}
```

**Returns:** `void`

#### `profile_update`

Updates an existing profile's name and fields by ID.

**Parameters:**

```typescript
{
  id: string;
  name: string;
  fields: any;          // Key-value fields (serde_json::Value)
}
```

**Returns:** `void`

#### `profile_delete`

Removes a profile by ID. Clears `active_id` in `ProfilesFile` if the deleted profile was the active one.

**Parameters:**

```typescript
{
  id: string;
}
```

**Returns:** `void`

#### `profile_set_active`

Sets a profile as active by updating `active_id` in `ProfilesFile`.

**Parameters:**

```typescript
{
  id: string;
}
```

**Returns:** `void`

### Story Management

Story commands manage story definitions stored as individual JSON files.

**File:** `engine/src/src/commands/story.rs`

#### `story_get`

Returns a story by ID.

**Parameters:**

```typescript
{
  storyId: string;
}
```

**Returns:** `StoryEntry`

```typescript
interface StoryEntry {
  version: 1;
  id: string;
  title: string;
  description?: string;
  config: Record<string, unknown>;
  created_at: string;
  updated_at: string;
}
```

#### `story_save`

Creates or replaces a story file at `{appData}/stories/{id}.json`.

**Parameters:**

```typescript
{
  story: StoryEntry;
}
```

**Returns:** `void`

### Lore Queries

Lore commands search lore entries for a given story.

**File:** `engine/src/src/commands/lore.rs`

#### `lore_query`

Searches lore entries for a story by title or content using case-insensitive substring matching (LIKE).

**Parameters:**

```typescript
{
  storyId: string;
  query: string;
}
```

**Returns:** `LoreQueryResult`

```typescript
interface LoreQueryResult {
  results: LoreEntry[];
}

interface LoreEntry {
  id: string;
  story_id: string;
  title: string;
  content: string;
  tags?: string[];
}
```

## Listening for Events

For long-running operations (LLM streaming), use `streamLlmFull()` from `gui/src/composables/useLlmStream.ts`. It handles event registration, fire-and-forget invocation of `invoke('prompt', ...)`, queue draining, and cleanup automatically.

```typescript
import { streamLlmFull } from '~/composables/useLlmStream';

for await (const event of streamLlmFull({ persona: PERSONA_PLATFORM, messages })) {
  if (event.type === 'text') { /* append text */ }
  if (event.type === 'reasoning') { /* append reasoning */ }
  if (event.type === 'error') { /* handle error */ }
  if (event.type === 'done') {
    console.log('Finish:', event.finishReason, 'Usage:', event.usage);
  }
  if (event.type === 'tool-call') { /* event.data = { id, tool, args } JSON */ }
  if (event.type === 'tool-result') { /* event.data = { id, tool, result } JSON */ }
}
```

**Rule:** Always use `streamLlmFull()` from `gui/src/composables/useLlmStream.ts` for LLM streaming. Never duplicate event listening logic in components. For AI SDK integration, use `createTauriModel()` from `~/utils/tauriLanguageModel` instead.

## Adding New Commands

### In Rust

1. Create a new function with `#[tauri::command]` and `#[specta::specta]` in `engine/src/src/commands/*.rs`
2. Ensure all parameter/return structs derive `specta::Type` (use `#[specta(type = Any)]` for `serde_json::Value` fields, or the `JsonAny` wrapper for command parameters)
3. Register it in `engine/src/src/lib.rs` via `tauri::generate_handler![]` (the tauri-specta `Builder` picks it up automatically)
4. Bindings are auto-generated at app startup in debug builds to `gui/src/bindings.ts`

```rust
#[tauri::command]
#[specta::specta]
async fn my_command(param: String) -> Result<String, String> {
    Ok(format!("Received: {}", param))
}
```

### From Frontend

```typescript
import { commands } from '~/bindings';
import { unwrap } from '~/utils';

const result = await unwrap(commands.myCommand({ param: 'hello' }));
```

## Related Documentation

- [Code Conventions](./code-conventions.md) - Import patterns and async functions
- [Data Storage](./database-schema.md) - JSON file formats and storage layout
- [Project Structure](./project-structure.md) - File organization for commands
- [Frontend Architecture](./frontend-architecture.md) - How the frontend uses these commands
