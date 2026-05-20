# Agents Documentation

High-level overview for AI agents working on the NovelCraft project. For comprehensive documentation, see the [detailed docs](./docs/).

**IMPORTANT:** Whenever you make changes, assert validity by running `just check`, then summarize & document the changes with the `@docs-writer` subagent.

## Architecture Overview

NovelCraft is a **Tauri v2 desktop app** — fully offline-first, single-user, no server.

- **Rust backend** (`engine/src/`) handles LLM proxy (HTTP streaming via `reqwest`, SSE parsing via `util.rs`) and file operations (export/import, file dialogs)
- **Vue 3 frontend** (`gui/src/`) runs in a webview via Vite — manages gameplay state, LLM orchestration, and all UI
- **Local SQLite** via `tauri-plugin-sql` — structured gameplay data (stories, sessions, pages, state snapshots, lore entries, profiles) lives in a local `.db` file
- **Persistent key/value store** via `tauri-plugin-store` — simple flags and preferences (e.g. onboarding completed) stored as JSON on disk. Used instead of SQLite for single-row, non-queryable data.
- **No server, no auth, no PostgreSQL** — single-user desktop application
- **LLM calls** go through Rust Tauri commands — `useLlmStream` internally uses `ConversationalArchetype` from `@stegakir/aikit` with `createTauriModel()` (which implements `LanguageModelV3` from `@ai-sdk/provider`). The AI SDK bridge calls `invoke('prompt', ...)` and maps Tauri events (`llm:text`, `llm:reasoning`, `llm:tool_call`, `llm:error`, `llm:done`) to AI SDK stream parts. Request ID scoping is managed internally by `TauriLanguageModel.doStream()`.
- **Story sharing** is file-based — export/import JSON files via native file dialogs
- **Build orchestration** is via a root `justfile` — no top-level `package.json`

## Quick Reference

### Project Structure

```
novelcraft/
├── justfile                # Build orchestration (just commands)
├── engine/                 # Rust backend (Tauri v2)
│   └── src/
│       ├── src/
│       │   ├── main.rs     # Entry point
│       │   ├── lib.rs      # App builder, plugin registration, command handler
│       │   ├── util.rs     # SSE stream parsing (StreamEvent enum, process_stream())
│       │   ├── commands/
│       │   │   ├── mod.rs  # Module barrel
│       │   │   ├── llm.rs  # LLM proxy (HTTP streaming via reqwest, delegates SSE to util)
│       │   │   └── fs.rs   # File export/import, file dialogs
│       │   └── infer/
│       │       ├── mod.rs  # Module barrel (pub mod api)
│       │       └── api.rs  # OpenAI API types (request/response structs for SSE)
│       ├── Cargo.toml      # Rust dependencies
│       └── tauri.conf.json # Tauri configuration
├── gui/                    # Vue 3 frontend (Vite + Vue Router)
│   ├── src/
│   │   ├── main.ts         # App entry, mounts Vue + Router
│   │   ├── App.vue         # Root component — switches between Onboarding and Main
│   │   ├── Main.vue        # Main app shell (sidebar, router view, shortcuts, dialogs)
│   │   ├── Onboarding.vue  # Step-based first-run onboarding flow
│   │   ├── env.d.ts        # Global type declarations & auto-imports
│   │   ├── pages/          # Vue Router pages
│   │   ├── components/     # Vue components
│   │   ├── composables/    # Vue composables
│   │   ├── db/             # Drizzle ORM (schema + instance)
│   │   ├── utils/          # Frontend utilities
│   │   ├── gameplay/       # Game modules (barrel via index.ts)
│   │   ├── prompts.ts      # Prompts & personas (single source of truth)
│   │   ├── router/         # Vue Router configuration
│   │   └── assets/css/     # Open Props CSS
│   ├── drizzle.config.ts      # Drizzle Kit configuration (schema, out, dialect)
│   ├── drizzle/                # Generated migration SQL files + meta/_journal.json
│   ├── index.html          # Vite entry HTML
│   ├── vite.config.ts      # Vite config with ~ alias
│   ├── tsconfig.json       # TypeScript config with ~/* paths
│   └── package.json        # Dependencies
└── docs/                   # Comprehensive documentation
```

**Detailed docs:** [Project Structure](./docs/project-structure.md)

---

## Key Conventions Summary

### File Organization

| Category | Location | Pattern |
|----------|----------|---------|
| Pages | `gui/src/pages/` | `[route].vue`, `[param].vue` for dynamic |
| Components | `gui/src/components/` | PascalCase `.vue` |
| Composables | `gui/src/composables/` | `use*.ts` |
| Utilities | `gui/src/utils/` | Named exports |
| Router | `gui/src/router/` | `index.ts` with `createRouter()` |
| Gameplay Modules | `gui/src/gameplay/` | Named exports, barrel via `index.ts` |
| Prompts | `gui/src/prompts.ts` | Single source of truth |
| Rust Commands | `engine/src/src/commands/` | One file per domain (`llm.rs`, `fs.rs`) |
| Rust Utilities | `engine/src/src/util.rs` | SSE stream parsing (`StreamEvent`, `process_stream`) |
| DB Schema | `gui/src/db/` | Drizzle ORM schema (`schema.ts`) + instance (`index.ts`) |

**Detailed docs:** [Code Conventions](./docs/code-conventions.md)

### Import Patterns

**Always use the `~/` alias — never relative `../` paths.**

| Alias | Resolves to | Use in |
|-------|------------|----------|
| `~/` | `gui/src/` | All frontend code (pages, components, composables, utils, gameplay) |

- **Auto-imports**: A custom Vite plugin (`gui/vite-plugins/auto-import.ts`) automatically injects the following identifiers into all `.ts` and `.vue` (`<script setup>`) files at build time. **Never import these manually** — it causes duplicate import conflicts:
  - From `vue`: `ref`, `reactive`, `computed`, `watch`, `readonly`, `onMounted`, `onUnmounted`, `nextTick`
  - From `vue-router`: `useRoute`, `useRouter`
- Type declarations for these are in `gui/src/env.d.ts` (for TypeScript). The Vite plugin handles runtime injection.
- **Components**: Import explicitly with `~/components/...` paths (not auto-imported)
- **No `@/`, `#shared/`, or `#server/` aliases** — those were removed

### AI / Model Configuration

Models are configured in Rust, persisted to disk as JSON.

- **Rust side**: `engine/src/src/commands/llm.rs` — `init_models()` loads from `{app_data_dir}/models.json` or falls back to defaults
- **Frontend side**: Call `invoke('list_models')` to read, `invoke('save_models', { models })` to write
- Each model entry maps a **usage ID** (e.g. `"storyteller"`, `"suggestions"`) → `{ model_id, base_url, api_key? }`, where `model_id` is the actual LLM API model identifier (e.g. `"zai-org/glm-4.6v-flash"`)
- Default models point to `http://localhost:1234/v1` (local LLM server)

### Agent / LLM Integration

**All LLM calls go through the Rust backend via Tauri commands.**

- **`invoke('prompt', { request })`** — takes `{ model, messages[], persona?, context?, request_id?, tools? }` and streams the response. The invoke is fire-and-forget (not awaited) because the Rust command resolves only after `llm:done` is emitted.
- Rust backend calls the OpenAI-compatible chat completions API, parses SSE frames via `util::process_stream()`, and emits Tauri events:
  - `llm:text` — text content chunk
  - `llm:reasoning` — reasoning/thinking chunk
  - `llm:tool_call` — tool/function call streaming delta (`{ index, id?, name?, arguments_delta }`)
  - `llm:error` — error message
  - `llm:done` — stream complete (payload: `{ finish_reason, usage? }`)
- Events are scoped with `request_id` when provided: `llm:{event}:{request_id}`. This enables concurrent LLM streams without event collisions.
- **Frontend composable**: `gui/src/composables/useLlmStream.ts` creates a `MemoryMessageStore` + `Conversation` from the messages array, then uses `ConversationalArchetype.prompt()` with `createTauriModel(modelId)`. The AI SDK stream parts are mapped to `StreamEvent` objects (`text`, `reasoning`, `error`, `done`, `tool-call`, `tool-result`). Tool-call events yield `{ id, tool, args }` as JSON. Tool-result events yield `{ id, tool, result }` as JSON.
- **AI SDK bridge**: `gui/src/utils/tauriLanguageModel.ts` — `createTauriModel(modelId)` returns a `LanguageModelV3` implementation that bridges Tauri events to Vercel AI SDK stream parts. Request ID scoping is managed internally by `TauriLanguageModel.doStream()`.

**Prompts and personas** are defined in `gui/src/prompts.ts`.
Always import from `~/prompts` & maintain them there as a single source of truth.

**Important terminology:** A "persona" is ONLY the system prompt passed as the `persona` parameter to the LLM call — it defines who the agent *is*. The sole persona used throughout the app is `PERSONA_PLATFORM`. Everything else — scene instructions (`SYSTEM_VIGNETTE_OPEN`), steering notes (`SYSTEM_STEER`), editor requests (`SYSTEM_INSTRUCT`), page-level `system` fields — are **NOT** personas. They are regular messages with `author: 'system'` injected into the conversation history to guide the agent's behavior.

### Persistent Key/Value Store — tauri-plugin-store

Simple flags, preferences, and single-row settings are stored in **tauri-plugin-store** (a JSON file on disk) rather than SQLite tables. A single-row SQLite table is wasteful overhead when a key/value store suffices.

- **Frontend**: `LazyStore` from `@tauri-apps/plugin-store` — lazy-loads a store file (e.g. `app.json`) and provides `get()`/`set()`/`has()` methods
- **Store file**: `app.json` in the Tauri app data directory
- **Rust side**: `tauri-plugin-store = "2"` in `Cargo.toml`, `.plugin(tauri_plugin_store::Builder::default().build())` in `lib.rs`
- **Capabilities**: `store:default` in `engine/capabilities/default.json`
- **Current usage**: Onboarding completed flag (`onboarding_completed` boolean in `app.json`), managed by `gui/src/composables/useOnboarding.ts`

**Convention**: Use `tauri-plugin-store` for simple key/value pairs (flags, preferences, single-row settings). Use SQLite tables only for structured, multi-row, queryable data.

### Client-Side Data — SQLite via Drizzle ORM

Gameplay state (stories, sessions, pages, state snapshots, lore entries, profiles) is stored in **local SQLite** via `tauri-plugin-sql`, accessed through **Drizzle ORM**.

- **DB module**: `gui/src/db/` — Drizzle ORM schema definitions and instance
  - `schema.ts` — Drizzle table definitions for all 6 SQLite tables (`localStories`, `localSessions`, `localPages`, `localStateSnapshots`, `localLoreEntries`, `localProfiles`). Column property names are camelCase mapping to snake_case SQL columns.
  - `index.ts` — Drizzle instance using `drizzle-orm/sqlite-proxy` adapter bridged to `@tauri-apps/plugin-sql`. Exports `db` (the Drizzle instance), `schema`, and re-exports all individual table objects. Runs drizzle-kit migrations on first connection (lazy singleton), with baseline detection for pre-migration databases.
- **Import pattern**: `import { db, localProfiles } from '~/db';` plus operators from `drizzle-orm` (e.g. `eq`, `desc`, `like`, `and`, `or`)
- **Composable**: `gui/src/composables/useProfiles.ts` — wraps `local_profiles` table; exposes `profiles`, `activeProfile`, `create`, `update`, `remove`, `setActive`, `init`; auto-creates a default profile on first use (max 5)
- **Mode**: Local-only, no sync
- **DB file**: `sqlite:novelcraft.db` (path managed by Tauri plugin)
- **Dependencies**: `@tauri-apps/plugin-sql`, `drizzle-orm`, `drizzle-kit` (dev)
- **Migrations**: Managed via `drizzle-kit`. Migration SQL files in `gui/drizzle/` are bundled at build time via `import.meta.glob` and applied on first DB connection. Baseline detection handles databases created before the migration system existed.
- **No `db.transaction()`**: The `drizzle-orm/sqlite-proxy` adapter simulates transactions by sending raw `BEGIN`/`COMMIT` SQL, but `@tauri-apps/plugin-sql` uses a `sqlx::Pool<Sqlite>` with default `max_connections = 10`. Each `pool.execute()` within the "transaction" can route to a different pool connection, so the transaction context is lost. This causes silent data loss. **Always use individual `db.insert()`/`db.update()`/`db.delete()` calls instead of `db.transaction()`.**

**Profile fields in prompts:** The active profile's fields are injected into story/gameplay LLM calls (vignette opening, write, steer, instruct) as a `[Player profile]` block in the context message via `buildProfileContext()` in `gui/src/composables/useGame.ts`. Profile fields are NOT injected into suggestion prompts or story metadata prompts.

```typescript
// Drizzle ORM query in any composable/component
import { db, localSessions } from '~/db';
import { eq, desc } from 'drizzle-orm';

const sessions = await db.select().from(localSessions).orderBy(desc(localSessions.createdAt));

// Insert
import { localPages } from '~/db';
await db.insert(localPages).values({
  id, sessionId, response, createdAt: new Date().toISOString(),
});

// Multi-step write — use individual calls, NOT db.transaction()
// (sqlite-proxy + sqlx connection pool breaks transaction semantics)
await db.delete(localPages).where(eq(localPages.sessionId, sessionId));
await db.delete(localSessions).where(eq(localSessions.id, sessionId));
```

### LLM Streaming — useLlmStream

All LLM streaming goes through `gui/src/composables/useLlmStream.ts`.

- `streamLlm(options)` — async generator yielding text chunks only
- `streamLlmFull(options)` — async generator yielding full `StreamEvent` objects (`text`, `reasoning`, `error`, `done`, `tool-call`, `tool-result`). Done events include `finishReason` and `usage` fields. Tool-call events yield `{ id, tool, args }` as JSON. Tool-result events yield `{ id, tool, result }` as JSON.
- `StreamLlmOptions` accepts optional `model`, `context`, and `tools` parameters. The `context` is passed to `ConversationalArchetype` as the context parameter. The `tools` is an optional Vercel AI SDK `ToolSet` (built via `GameplayModuleRegistry.getToolSet()`) passed through to `ConversationalArchetype` for gameplay module tool use.
- Replaces any inline Tauri event listening — never duplicate event logic in components
- Internally creates a `MemoryMessageStore` + `Conversation` from the messages array, then uses `ConversationalArchetype.prompt()` with `createTauriModel(modelId)`. AI SDK `TextStreamPart` events are mapped to `StreamEvent` objects (`text-delta` → text, `reasoning-delta` → reasoning, `finish` → done with usage, `error` → error, `tool-call` → tool-call with `{ id, tool, args }` JSON, `tool-result` → tool-result with `{ id, tool, result }` JSON).
- Request ID scoping is managed internally by `TauriLanguageModel.doStream()` — no need to pass `requestId` from callers.

```typescript
import { streamLlmFull } from '~/composables/useLlmStream';

for await (const event of streamLlmFull({ persona, messages })) {
  if (event.type === 'text') { /* append text */ }
  if (event.type === 'reasoning') { /* append reasoning */ }
  if (event.type === 'error') { /* handle error */ }
  if (event.type === 'done') { /* event.finishReason, event.usage available */ }
  if (event.type === 'tool-call') { /* event.data = { id, tool, args } JSON */ }
  if (event.type === 'tool-result') { /* event.data = { id, tool, result } JSON */ }
}
```

### Gameplay Module System (`gui/src/gameplay/`)

Modules define gameplay mechanics via tools exposed to the LLM.

**Core types** (`gameplayModule.ts`):
- `GameplaySession` — simplified to `{ storyId, sessionId, state: Record<string, unknown> }`. No `modules` field. State is a flat map from module type to module-specific data. Managed by `useVignette` (via `getGameplaySession()`) rather than a dedicated composable.
- `GameplayModuleContext` — has `session`, `module`, `state` fields (access `storyId`/`sessionId` via `ctx.session.storyId`/`ctx.session.sessionId`)
- `ToolResult<S>` — `{ success: true; state?: S; response?: string } | { success: false; error: string }`. When `state` is omitted on success, the draft mutations from immer are used. The `response` field is used by query-only tools (like lore) to return data to the LLM without mutating state.
- `ExecuteToolResult` — `{ success: true; newState: unknown; response: string }`. Returned by `executeTool()`.
- `GameplayModuleRegistry` — class with `get()`, `getAll()`, `executeTool()`, `getToolSet()` methods. `executeTool()` is the single method encapsulating the init-draft-execute-finalize sequence (handles `init()` fallback, immer `createDraft`/`finishDraft`, and state finalization). Both `getToolSet()` (live tool execution during LLM streaming) and `replay()` in `useVignette` delegate to `executeTool()`. Tool names in module definitions are unprefixed (e.g. `'move'`); `getToolSet()` auto-prefixes them to `${modType}::${toolDef.name}`. The `onToolCall` callback receives the full prefixed key (see below)
- `createDefaultRegistry()` — factory that creates a registry pre-loaded with `NPCModule`, `PlanModule`, `LoreModule`. Uses `GameplayModuleRegistry` constructor (no `register()` method).
- `defineGameplayModule()` — factory with `.withTool()` builder pattern for adding tools to a module. Requires an `init()` method returning the default state (supports `MaybePromise`).
- `MaybePromise<T>` — local type alias in `gameplayModule.ts` for `T | Promise<T>`
- `OnToolCall` — type alias for tool call callback
- `Subagent` — interface for sub-agent definitions
- `toolCallRecordSchema` — zod schema for tool call records stored in pages
- `toolOk(state?, opts?)` — `state` is optional. `toolOk()` means success with no state override (draft mutations apply). `toolOk(newState)` means success, override entire state. Accepts optional `{ response?: string }` as second arg
- `toolErr(error)` — returns a failure result

**Immer state transitions** (centralized in `GameplayModuleRegistry.executeTool()`):

Tool execution uses `immer` for immutable state updates. Both `getToolSet()` (live tool execution during LLM streaming) and `replay()` in `useVignette` delegate to `executeTool()`, which handles the full init-draft-execute-finalize sequence:
1. When module state is undefined, `module.init()` is called to produce default state
2. `createDraft(base)` creates a mutable draft proxy
3. Draft is passed as `ctx.state` to the tool — the tool mutates `ctx.state` directly
4. If `result.state` is provided, it overrides the draft entirely
5. If `result.state` is undefined, `finishDraft(draft)` produces the new immutable state
6. On error (tool returns `toolErr()`), `executeTool()` throws — the draft is discarded (no state change). `getToolSet()` catches this and wraps it in an `Error: ...` string for the AI SDK

Modules use direct draft mutation instead of spread operators:
- `npcModule.ts`: `state.npcs[name]!.location = destination; return toolOk();`
- `planModule.ts`: `state.roadmap = roadmap; return toolOk();`
- `loreModule.ts`: `return toolOk(undefined, { response: ... });` (query-only, no state mutation)

**Dependency:** `immer` ^11.1.6

**Registered modules** (via `createDefaultRegistry()`):
- `NPCModule` — NPC management, tool `move` (defined unprefixed, auto-prefixed to `npc::move`)
- `PlanModule` — `getKnowledge()` returns `{ roadmap }`, tool `updateRoadmap` (auto-prefixed to `plan::updateRoadmap`)
- `LoreModule` — no knowledge injection, tool `query` (auto-prefixed to `lore::query`) queries `local_lore_entries` via LIKE search

**Deleted modules:** `eventModule.ts`, `graphMapModule.ts`, `systemPromptModule.ts` — removed during the module system redesign.

### Snapshot Lifecycle

State snapshots in `local_state_snapshots` enable undo/fork by capturing module state at each page:

- **Session creation** → snapshot 0 with empty state `{}`
- **After each page's tool calls** → head snapshot updated in place (or new snapshot inserted at checkpoint boundaries)
- **Every 100 pages** → copy current head as checkpoint before updating
- **Fork** → delete snapshots with `page_index >= fork_index`, recompute head from youngest snapshot before `fork_index` via tool call replay through `registry.executeTool()` (with `init()` fallback for uninitialized module state), then `push()` to create a new page
- **State is immutable** — immer drafts are used for convenience during tool execution and replay (inside `executeTool()`), but the canonical state is the snapshot data
- **Initial module state** is always empty (`{}`); first tool call populates what's needed via `init()` fallback (forward-compatible with module upgrades). `executeTool()` applies this fallback when module state is undefined.

### LLM Helpers — buildMessages

`buildMessages()` is defined in `gui/src/composables/useGame.ts` (moved from deleted `llmHelpers.ts`):
- `buildMessages()` now accepts optional `gameplaySession?: GameplaySession`
- When provided, calls each module's `getKnowledge()` and injects results into context under `context.modules`
- `buildProfileContext()` — injects active profile fields as `[Player profile]` block

### useVignette — Push/Fork Coordination

`useVignette` manages a single vignette session and exposes a 2-step push/fork pattern:

- **`push({ prompt?, system? })`** — inserts a page into the DB/reactive array and returns a `PromptUpdater` function. The updater is called later with `(response, toolCalls, state)` to finalize the page with the LLM's response and state transition.
- **`fork({ pageIndex, system?, prompt? })`** — based on `push()`: truncates pages >= pageIndex (DB + reactive), deletes snapshots >= pageIndex, loads the youngest snapshot before pageIndex, replays tool calls from surviving pages via `registry.executeTool()` (with `init()` fallback for uninitialized module state), then calls `push()` to create a new page (returning its updater). System/prompt follow null=clear, undefined=keep-old, otherwise=override semantics.
- **`update({ pageIndex, system?, prompt?, response? })`** — edits page text without AI involvement. Null clears, undefined keeps existing.
- **`getGameplaySession()`** — returns a `GameplaySession` derived from the current snapshot.
- **`snapshot`** — readonly ref exposing the current state snapshot.
- **`run()`** (from `useGame`) is stateless — it takes a `GameplaySession`, builds messages, streams the LLM, and returns `{ response, toolCalls, state }`. It does NOT persist anything. Consumers coordinate persistence by calling the `PromptUpdater` returned by `push()`/`fork()`.

### TypeScript — No `as any`

**Never use `as any` casts.** If a type incompatibility arises, fix the root cause:

- Widen or narrow the source/target types (e.g. `Record<string, unknown>` instead of `unknown`)
- Fix schema types
- Remove unnecessary type wrappers (e.g. `DeepReadonly` that creates cascading `Readonly<unknown>` constraints)
- Use targeted type assertions like `as ExpectedType` when crossing package boundaries
- If a genuine cross-package type mismatch exists, **inform the user** rather than silently casting to `any`

### Module Type

ESM modules (`"type": "module"` in gui/package.json)

### Styling — Open Props

The project uses [Open Props](https://open-props.style/) as its styling foundation. It provides CSS custom property design tokens for spacing, color, typography, shadows, radii, animations, and more.

- **Global styles**: `gui/src/assets/css/` — imports `open-props` (tokens) and `normalize` (reset)
- **Usage**: Reference tokens via `var(--size-3)`, `var(--gray-7)`, `var(--radius-2)`, `var(--shadow-2)`, `var(--font-size-4)`, etc.
- **Semantic tokens**: `--text-1`/`--text-2`, `--surface-1`..`--surface-4` for colors that adapt to dark mode
- **Brand**: `--brand-gradient` defined in global CSS for the app's gradient
- **Rules**:
  - Never hardcode colors, spacing, radii, shadows, or font sizes — use Open Props tokens
  - Use logical properties (`inline-size`/`block-size`, `margin-block-start`, etc.)
  - Write scoped `<style scoped>` in Vue components
  - No class-name frameworks (Tailwind, etc.)

---

## Common Tasks

### Running the Application

All commands are managed by the root `justfile`. Run `just` with no arguments to list available recipes.

```bash
# Frontend only (Vite dev server on :5173)
just dev-frontend

# Engine (cargo tauri dev, requires frontend already running)
just dev-engine

# Build frontend (Vite production build)
just build-frontend

# Build engine (cargo tauri build)
just build-engine

# Build both frontend + engine
just build
```

### Type Checking & Linting

```bash
# TypeScript (vue-tsc --noEmit in gui/)
just typecheck

# Typecheck + cargo check
just check

# Cargo clippy
just clippy

# Cargo fmt / fmt --check
just fmt
just fmt-check
```

---

## Database Query Patterns

### Local Database (SQLite — via Drizzle ORM)

All data access uses Drizzle ORM through `db` and table references from `gui/src/db/`.

```typescript
import { db, localSessions, localPages, localStateSnapshots } from '~/db';
import { eq, desc, and, like, or } from 'drizzle-orm';

// Query sessions
const sessions = await db.select().from(localSessions).orderBy(desc(localSessions.createdAt));

// Query with filter
const pages = await db.select().from(localPages)
  .where(eq(localPages.sessionId, sessionId))
  .orderBy(localPages.createdAt);

// Insert
await db.insert(localPages).values({
  id, sessionId, system, prompt, response, createdAt: new Date().toISOString(),
});

// Update
await db.update(localSessions)
  .set({ title: newTitle, updatedAt: new Date().toISOString() })
  .where(eq(localSessions.id, sessionId));

// Delete
await db.delete(localSessions).where(eq(localSessions.id, sessionId));

// Multi-step write — use individual calls, NOT db.transaction()
// sqlite-proxy + sqlx Pool connection routing breaks transaction semantics
await db.delete(localPages).where(eq(localPages.sessionId, sessionId));
await db.delete(localSessions).where(eq(localSessions.id, sessionId));
```

**Detailed docs:** [Database Schema](./docs/database-schema.md)

---

## Tauri Commands

### Available Commands

**LLM Proxy:**

| Command | Parameters | Returns | Description |
|---------|-----------|---------|-------------|
| `prompt` | `{ request: { model, messages[], persona?, context?, request_id?, tools? } }` | `void` (emits events) | Stream LLM response |
| `list_models` | none | `Record<string, ModelConfig>` | Get configured models |
| `save_models` | `{ models: Record<string, ModelConfig> }` | `void` | Save model configuration |

**File Operations:**

| Command | Parameters | Returns | Description |
|---------|-----------|---------|-------------|
| `export_session` | `{ session_id, file_path, data: ExportData }` | `void` | Write session to file |
| `import_session` | `{ file_path }` | `ExportData` | Read session from file |
| `pick_file` | `{ filters? }` | `string \| null` | Open native file picker |
| `pick_folder` | none | `string \| null` | Open native folder picker |

### Calling Commands from Frontend

```typescript
import { invoke } from '@tauri-apps/api/core';

// LLM streaming (use the composable — never call invoke('prompt', ...) directly)
// import { streamLlmFull } from '~/composables/useLlmStream';

// Get/save models
const models = await invoke<Record<string, ModelConfig>>('list_models');
await invoke('save_models', { models: updatedModels });

// File dialogs
const filePath = await invoke<string | null>('pick_file', {
  filters: [{ name: 'JSON', extensions: ['json'] }],
});

// Export/import
await invoke('export_session', { session_id: id, file_path: filePath, data: exportData });
const imported = await invoke<ExportData>('import_session', { file_path: filePath });
```

### Listening for LLM Events

```typescript
import { streamLlmFull } from '~/composables/useLlmStream';

for await (const event of streamLlmFull({ persona: PERSONA_PLATFORM, messages })) {
  if (event.type === 'text') { /* append text */ }
  if (event.type === 'reasoning') { /* append reasoning */ }
  if (event.type === 'error') { /* handle error */ }
  if (event.type === 'done') { /* event.finishReason, event.usage available */ }
  if (event.type === 'tool-call') { /* event.data = { id, tool, args } JSON */ }
  if (event.type === 'tool-result') { /* event.data = { id, tool, result } JSON */ }
}
```

**Do not use `listen('llm:*', ...)` directly** — always go through `useLlmStream`. For AI SDK integration, use `createTauriModel()` from `~/utils/tauriLanguageModel` instead.

**Detailed docs:** [API Routes](./docs/api-routes.md)

---

## Frontend Patterns

### Routing (Vue Router)

```typescript
// Navigation
const router = useRouter();
router.push('/vignettes');
router.push({ name: 'vignette', params: { id: sessionId } });

// Route params
const route = useRoute();
const id = route.params.id;
```

**Routes** (defined in `gui/src/router/index.ts`):

| Path | Name | Component |
|------|------|-----------|
| `/` | home | `gui/src/pages/index.vue` |
| `/vignettes` | vignettes | `gui/src/pages/vignettes/index.vue` |
| `/vignettes/:id` | vignette | `gui/src/pages/vignettes/[id].vue` |
| `/builder` | builder | `gui/src/pages/builder.vue` |
| `/settings` | settings | `gui/src/pages/settings.vue` |

### Component Props

```typescript
interface Props {
  story: {
    id: string;
    title: string;
  };
}

defineProps<Props>();
```

### LLM Streaming

```typescript
import { streamLlmFull } from '~/composables/useLlmStream';

for await (const event of streamLlmFull({ persona: PERSONA_PLATFORM, messages })) {
  if (event.type === 'text') { /* append text */ }
  if (event.type === 'error') { /* handle error */ }
  if (event.type === 'done') { /* event.finishReason, event.usage available */ }
  if (event.type === 'tool-call') { /* event.data = { id, tool, args } JSON */ }
  if (event.type === 'tool-result') { /* event.data = { id, tool, result } JSON */ }
}
```

### Local Data Access

```typescript
import { db, localSessions } from '~/db';
import { eq, desc } from 'drizzle-orm';

const sessions = await db.select().from(localSessions).orderBy(desc(localSessions.createdAt));
```

**Detailed docs:** [Frontend Architecture](./docs/frontend-architecture.md)

---

## Agent Guidelines

### When to Add a Tauri Command

- Frontend needs access to a system capability (file system, native dialogs, etc.)
- New LLM-related functionality that requires Rust-side HTTP handling
- Operations that should run outside the webview sandbox

**Process:**
1. Create a new function with `#[tauri::command]` in the appropriate `engine/src/src/commands/*.rs` file
2. Register it in `engine/src/src/lib.rs` via `tauri::generate_handler![]`
3. Call from frontend via `invoke('command_name', { params })`

### When to Modify Local DB Schema

- Client-side gameplay state needs new tables or columns
- Vignette, session, or profile data structures change

**Process:**
1. Add/modify table definitions in `gui/src/db/schema.ts`
2. Run `just generate-migration` — this generates a new numbered SQL migration in `gui/drizzle/`
3. Migration is automatically applied on next app launch (bundled via `import.meta.glob`, run by `runMigrations()` in `gui/src/db/index.ts`)
4. For existing databases created before the migration system, baseline detection marks all migrations as applied without re-running them

### When to Create a Component

- Reusable UI patterns across multiple pages
- Complex logic that should be isolated
- Shared functionality (cards, modals, forms)

**Process:**
1. Create `.vue` file in `gui/src/components/`
2. Use `defineProps<T>()` for TypeScript props
3. Use scoped styles to avoid conflicts
4. Import explicitly: `import MyComponent from '~/components/MyComponent.vue'`

### When to Add a Composable

- Shared reactive state or logic across components/pages
- Wrapping Tauri APIs or external libraries
- Data access patterns (local DB queries)

**Process:**
1. Create `use*.ts` file in `gui/src/composables/`
2. Export a composable function using Vue's `ref`/`computed`/`onMounted` etc.
3. If it should be auto-imported, add its type declaration to `gui/src/env.d.ts`

### When to Add a Page

- New route/view in the application

**Process:**
1. Create `.vue` file in `gui/src/pages/`
2. Add route to `gui/src/router/index.ts`
3. Use dynamic imports for code splitting: `component: () => import('~/pages/my-page.vue')`

---

## Justfile Commands

All build and development commands are in the root `justfile`.

| Recipe | Purpose |
|--------|---------|
| `just dev-frontend` | Start Vite dev server (`gui/`) |
| `just dev-engine` | Start `cargo tauri dev` (`engine/src/`, requires frontend running) |
| `just build-frontend` | Vite production build |
| `just build-engine` | cargo tauri build |
| `just build` | Both frontend + engine |
| `just typecheck` | `vue-tsc --noEmit` in `gui/` |
| `just check` | typecheck + `cargo check` |
| `just clippy` | cargo clippy in `engine/` |
| `just fmt` | cargo fmt in `engine/` |
| `just fmt-check` | cargo fmt --check in `engine/` |
| `just generate-migration` | Generate Drizzle Kit migration from schema changes (`cd gui && bunx drizzle-kit generate`) |

## Key Dependencies

### Frontend (gui/package.json)

| Package | Purpose |
|---------|---------|
| `vue` | UI framework |
| `vue-router` | Client-side routing |
| `@tauri-apps/api` | Tauri IPC (`invoke`, `listen`) |
| `@tauri-apps/plugin-sql` | SQLite database access |
| `@tauri-apps/plugin-store` | Persistent key/value store (flags, preferences) |
| `@tauri-apps/plugin-dialog` | Native file dialogs |
| `@tauri-apps/plugin-fs` | File system access |
| `@stegakir/aikit` | AI/LLM utilities (`ConversationalArchetype`, `Conversation`, `MemoryMessageStore`) |
| `@ai-sdk/provider` | Vercel AI SDK types (LanguageModelV3 interface) |
| `open-props` | CSS design tokens |
| `marked` | Markdown rendering |
| `zod` | Runtime validation |
| `immer` | Immutable state transitions in `GameplayModuleRegistry.executeTool()` (`createDraft`/`finishDraft`) |
| `drizzle-orm` | Type-safe SQLite query builder (ORM via `sqlite-proxy` adapter) |
| `drizzle-kit` | Schema migration generator (dev dependency) |
| `vite` | Build tool (dev dependency) |
| `@vitejs/plugin-vue` | Vite Vue plugin (dev dependency) |
| `vue-tsc` | Vue TypeScript checking (dev dependency) |

### Backend (engine/src/Cargo.toml)

| Crate | Purpose |
|-------|---------|
| `tauri` | Desktop app framework |
| `tauri-plugin-sql` | SQLite plugin |
| `tauri-plugin-store` | Persistent key/value store plugin |
| `tauri-plugin-dialog` | Native dialog plugin |
| `tauri-plugin-fs` | File system plugin |
| `reqwest` | HTTP client (streaming SSE) |
| `bytes` | Byte buffer utilities (SSE stream parsing) |
| `serde` / `serde_json` | Serialization |
| `tokio` | Async runtime |
| `futures` | Async stream utilities |
| `dirs` | Platform directories |

---

## Detailed Documentation Links

| Document | Description |
|----------|-------------|
| [Project Structure](./docs/project-structure.md) | Complete file organization and directory layout |
| [Code Conventions](./docs/code-conventions.md) | Code styling, imports, patterns, and best practices |
| [Database Schema](./docs/database-schema.md) | Local SQLite table definitions and query patterns |
| [API Routes](./docs/api-routes.md) | Tauri command documentation and usage patterns |
| [Frontend Architecture](./docs/frontend-architecture.md) | Pages, components, composables, and styling conventions |
