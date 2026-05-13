# API Routes

This document describes the Tauri commands available in Novelcraft. Since this is a desktop app with no server, all "API" functionality is exposed as Tauri commands invoked from the frontend webview via `@tauri-apps/api/core`.

## Overview

The application uses Tauri commands as its IPC layer. Commands are defined in `engine/src/src/commands/` and registered in `engine/src/src/lib.rs`. The frontend calls them via `invoke()`.

### Architecture

There is no HTTP server. All communication between the Vue frontend and Rust backend goes through Tauri's IPC:

- **LLM operations** — streaming proxy to OpenAI-compatible LLM APIs
- **File operations** — export/import sessions, native file/folder pickers
- **Data storage** — local SQLite via `tauri-plugin-sql` (no server database)

### Plugin Permissions

Tauri v2 plugin permissions are configured in `engine/capabilities/default.json`. This capability grants the main window access to the `sql`, `dialog`, and `fs` plugins (load, execute, select, open, save, read, write, etc.). When adding a new Tauri plugin, its permissions must be added here.

### Calling Commands from Frontend

```typescript
import { invoke } from '@tauri-apps/api/core';

// Simple command
const models = await invoke<Record<string, ModelConfig>>('list_models');

// Command with parameters
await invoke('save_models', { models: updatedModels });

// LLM streaming (use the composable — never call invoke('prompt', ...) directly)
// import { streamLlmFull } from '~/composables/useLlmStream';
```

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

**Model resolution:** The `model` field in the request is a **usage ID** (e.g. `"storyteller"`, `"suggestions"`). The Rust backend looks up the corresponding `ModelConfig` in the in-memory model registry (loaded from `{app_data_dir}/models.json`). The actual LLM API model identifier (`config.model_id`, e.g. `"zai-org/glm-4.6v-flash"`) is then sent as the `"model"` field in the API request body.

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

**`engine/src/util.rs`** — SSE stream parsing (extracted from `llm.rs`):
- `StreamEvent` — enum with variants: `Text(String)`, `Reasoning(String)`, `ToolCall { index, id, name, arguments_delta }`, `Done { finish_reason, usage }`
- `process_stream()` — async function that takes a byte stream and a callback `Fn(StreamEvent)`, parses SSE frames, and invokes the callback for each event. Tracks `finish_reason` and `usage` state internally and emits `Done` when `[DONE]` is received or the stream ends. Uses `futures::StreamExt` for byte stream iteration.

**`engine/src/infer/api.rs`** — OpenAI API wire types (imported by `llm.rs` via `use crate::infer::api::*`):
- `FunctionCall` / `ToolCall` — typed tool call structs matching the OpenAI format (`{ id, type, function: { name, arguments } }`), used in `LlmMessage::tool_calls`
- `ApiChatMessage`, `ApiToolFunction`, `ApiTool`, `StreamOptions`, `ChatCompletionRequest` — typed structs for building the API request body (replaces `serde_json::json!()` macro calls)
- `StreamResponse`, `StreamUsage`, `StreamChoice`, `StreamDelta`, `StreamToolCall`, `StreamFunctionDelta` — typed structs for SSE response parsing (replaces manual `.get()` chaining on `serde_json::Value`)

**`engine/src/commands/llm.rs`** — command-level and application types (remain in the command module). The `prompt` function builds the HTTP request, then calls `process_stream(response.bytes_stream(), &|event| match event { ... })` with a closure that maps `StreamEvent` variants to Tauri `emit_event` calls. SSE buffer/frame parsing logic was extracted to `util.rs`, making `llm.rs` focused on request building and error handling.
- `ModelConfig`, `LlmMessage`, `LlmTool`, `LlmToolCallDelta`, `LlmDonePayload`, `LlmUsage`, `LlmPromptRequest`, `UnreachableHost`, `PingHostRequest`

Fields still using `serde_json::Value`: `LlmTool::parameters` (JSON Schema passthrough to API) and `LlmPromptRequest::context` (dead code, kept for frontend compatibility).

**Backward compatibility:** All new fields (`persona` made optional, `request_id`, `tools`, `tool_call_id`, `tool_calls` on messages) use `#[serde(default)]` and are optional. Existing callers passing the old shape work unchanged. Events without `request_id` retain the same names as before.

**Frontend usage:** Always use `streamLlmFull()` or `streamLlm()` from `gui/src/composables/useLlmStream.ts`. The composable calls `invoke('prompt', ...)` without awaiting (fire-and-forget) because the Rust command resolves only after `llm:done` has been emitted. Never listen to Tauri events directly.

For AI SDK integration (e.g., `ConversationalArchetype` from `@stegakir/aikit`), use `createTauriModel(modelId)` from `gui/src/utils/tauriLanguageModel.ts` — this implements `LanguageModelV3` and handles scoped event routing automatically.

#### `list_models`

Returns the configured model registry.

**File:** `engine/src/src/commands/llm.rs`

**Parameters:** None

**Returns:** `Record<string, ModelConfig>`

```typescript
interface ModelConfig {
  model_id: string;       // Actual LLM API model identifier (e.g., 'zai-org/glm-4.6v-flash')
  base_url: string;       // API base URL (e.g., 'http://localhost:1234/v1')
  api_key?: string;       // Optional API key
}
```

The `Record<string, ModelConfig>` returned by `list_models` uses **usage IDs** as keys (e.g. `"storyteller"`, `"suggestions"`). Each value contains the actual LLM API model identifier in `model_id`.

#### `save_models`

Persists model configuration to disk (`{app_data_dir}/models.json`).

**File:** `engine/src/src/commands/llm.rs`

**Parameters:**

```typescript
{
  models: Record<string, ModelConfig>
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

- The `MutexGuard` on the model registry is dropped before any async HTTP operations to satisfy Rust's `Send` bounds.
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

1. Create a new function with `#[tauri::command]` in `engine/src/src/commands/*.rs`
2. Register it in `engine/src/src/lib.rs` via `tauri::generate_handler![]`

```rust
#[tauri::command]
async fn my_command(param: String) -> Result<String, String> {
    Ok(format!("Received: {}", param))
}
```

### From Frontend

```typescript
const result = await invoke<string>('my_command', { param: 'hello' });
```

## Related Documentation

- [Code Conventions](./code-conventions.md) - Import patterns and async functions
- [Database Schema](./database-schema.md) - Local SQLite table definitions
- [Project Structure](./project-structure.md) - File organization for commands
- [Frontend Architecture](./frontend-architecture.md) - How the frontend uses these commands
