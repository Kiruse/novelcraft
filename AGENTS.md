# Agents Documentation

High-level overview for AI agents working on the NovelCraft project. For comprehensive documentation, see the [detailed docs](./docs/).

**IMPORTANT:** Whenever you make changes, assert validity by running `just check`, then summarize & document the changes with the `@docs-writer` subagent.

## Architecture Overview

NovelCraft is a **Tauri v2 desktop app** — fully offline-first, single-user, no server.

- **Rust backend** (`engine/src/`) handles LLM proxy (HTTP streaming via `reqwest`) and file operations (export/import, file dialogs)
- **Vue 3 frontend** (`gui/src/`) runs in a webview via Vite — manages gameplay state, LLM orchestration, and all UI
- **Local SQLite** via `tauri-plugin-sql` — all gameplay data (sessions, pages, profiles, module runtime) lives in a local `.db` file
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
│       │   └── commands/
│       │       ├── mod.rs  # Module barrel
│       │       ├── llm.rs  # LLM proxy (HTTP streaming via reqwest)
│       │       └── fs.rs   # File export/import, file dialogs
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
│   │   ├── utils/          # Frontend utilities
│   │   ├── gameplay/       # Game modules (barrel via index.ts)
│   │   ├── prompts.ts      # Prompts & personas (single source of truth)
│   │   ├── router/         # Vue Router configuration
│   │   └── assets/css/     # Open Props CSS
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
- Rust backend calls the OpenAI-compatible chat completions API, parses SSE frames, and emits Tauri events:
  - `llm:text` — text content chunk
  - `llm:reasoning` — reasoning/thinking chunk
  - `llm:tool_call` — tool/function call streaming delta (`{ index, id?, name?, arguments_delta }`)
  - `llm:error` — error message
  - `llm:done` — stream complete (payload: `{ finish_reason, usage? }`)
- Events are scoped with `request_id` when provided: `llm:{event}:{request_id}`. This enables concurrent LLM streams without event collisions.
- **Frontend composable**: `gui/src/composables/useLlmStream.ts` creates a `MemoryMessageStore` + `Conversation` from the messages array, then uses `ConversationalArchetype.prompt()` with `createTauriModel(modelId)`. The AI SDK stream parts are mapped to `StreamEvent` objects (`text`, `reasoning`, `error`, `done`). Tool-related parts are handled transparently by `ConversationalArchetype`'s internal `ToolLoopAgent`.
- **AI SDK bridge**: `gui/src/utils/tauriLanguageModel.ts` — `createTauriModel(modelId)` returns a `LanguageModelV3` implementation that bridges Tauri events to Vercel AI SDK stream parts. Request ID scoping is managed internally by `TauriLanguageModel.doStream()`.

**Prompts and personas** are defined in `gui/src/prompts.ts`.
Always import from `~/prompts` & maintain them there as a single source of truth.

**Important terminology:** A "persona" is ONLY the system prompt passed as the `persona` parameter to the LLM call — it defines who the agent *is*. The sole persona used throughout the app is `PERSONA_PLATFORM`. Everything else — scene instructions (`SYSTEM_VIGNETTE_OPEN`), steering notes (`SYSTEM_STEER`), editor requests (`SYSTEM_INSTRUCT`), page-level `system` fields — are **NOT** personas. They are regular messages with `author: 'system'` injected into the conversation history to guide the agent's behavior.

### Client-Side Data — SQLite via tauri-plugin-sql

Gameplay state (sessions, pages, module runtime, profiles) is stored in **local SQLite** via `tauri-plugin-sql`.

- **Composable**: `gui/src/composables/useLocalDb.ts` — lazy-initializes the database, runs `CREATE TABLE IF NOT EXISTS`, exports `select<T>()` and `execute()` helpers
- **Composable**: `gui/src/composables/useProfiles.ts` — wraps `local_profiles` table; exposes `profiles`, `activeProfile`, `create`, `update`, `remove`, `setActive`, `init`; auto-creates a default profile on first use (max 5)
- **Mode**: Local-only, no sync
- **DB file**: `sqlite:novelcraft.db` (path managed by Tauri plugin)
- **Dependencies**: `@tauri-apps/plugin-sql`

**Profile fields in prompts:** The active profile's fields are injected into story/gameplay LLM calls (vignette opening, write, steer, instruct) as a `[Player profile]` block in the context message via `buildProfileContext()` in `gui/src/utils/llmHelpers.ts`. Profile fields are NOT injected into suggestion prompts or story metadata prompts.

```typescript
// Raw SQL query in any composable/component (auto-imported via env.d.ts)
const sessions = await select<{ id: string; title: string }>('SELECT id, title FROM local_sessions');

// Insert via execute
await execute('INSERT INTO local_pages (id, session_id, response, created_at) VALUES (?, ?, ?, ?)', [
  id, sessionId, response, new Date().toISOString(),
]);
```

### LLM Streaming — useLlmStream

All LLM streaming goes through `gui/src/composables/useLlmStream.ts`.

- `streamLlm(options)` — async generator yielding text chunks only
- `streamLlmFull(options)` — async generator yielding full `StreamEvent` objects (`text`, `reasoning`, `error`, `done`). Done events include `finishReason` and `usage` fields.
- `StreamLlmOptions` accepts optional `model` and `context` parameters. The `context` is passed to `ConversationalArchetype` as the context parameter.
- Replaces any inline Tauri event listening — never duplicate event logic in components
- Internally creates a `MemoryMessageStore` + `Conversation` from the messages array, then uses `ConversationalArchetype.prompt()` with `createTauriModel(modelId)`. AI SDK `TextStreamPart` events are mapped to `StreamEvent` objects (`text-delta` → text, `reasoning-delta` → reasoning, `finish` → done with usage, `error` → error). Tool-related parts are handled transparently by `ConversationalArchetype`'s internal `ToolLoopAgent`.
- Request ID scoping is managed internally by `TauriLanguageModel.doStream()` — no need to pass `requestId` from callers.

```typescript
import { streamLlmFull } from '~/composables/useLlmStream';

for await (const event of streamLlmFull({ persona, messages })) {
  if (event.type === 'text') { /* append text */ }
  if (event.type === 'reasoning') { /* append reasoning */ }
  if (event.type === 'error') { /* handle error */ }
  if (event.type === 'done') { /* event.finishReason, event.usage available */ }
}
```

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

### Local Database (SQLite — via tauri-plugin-sql)

All data access uses raw SQL through `select()` and `execute()` from `gui/src/composables/useLocalDb.ts`.

```typescript
// These are auto-imported globally (declared in env.d.ts)

// Query sessions
const sessions = await select<{ id: string; title: string; description: string | null }>(
  'SELECT id, title, description FROM local_sessions ORDER BY created_at DESC'
);

// Query with parameters
const pages = await select<{ id: string; response: string }>(
  'SELECT id, response FROM local_pages WHERE session_id = ? ORDER BY created_at',
  [sessionId]
);

// Insert
await execute(
  'INSERT INTO local_pages (id, session_id, system, prompt, response, created_at) VALUES (?, ?, ?, ?, ?, ?)',
  [id, sessionId, system, prompt, response, new Date().toISOString()]
);

// Update
await execute(
  'UPDATE local_sessions SET title = ?, updated_at = ? WHERE id = ?',
  [newTitle, new Date().toISOString(), sessionId]
);

// Delete
await execute('DELETE FROM local_sessions WHERE id = ?', [sessionId]);
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
}
```

### Local Data Access

```typescript
// select() and execute() are auto-imported globally
const sessions = await select<{ id: string; title: string }>(
  'SELECT id, title FROM local_sessions ORDER BY created_at DESC'
);
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
1. Add/modify the `CREATE TABLE IF NOT EXISTS` SQL in `gui/src/composables/useLocalDb.ts`
2. Schema changes apply on next app launch (tables created if not exist; column additions require manual `ALTER TABLE` or DB recreation)

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

## Key Dependencies

### Frontend (gui/package.json)

| Package | Purpose |
|---------|---------|
| `vue` | UI framework |
| `vue-router` | Client-side routing |
| `@tauri-apps/api` | Tauri IPC (`invoke`, `listen`) |
| `@tauri-apps/plugin-sql` | SQLite database access |
| `@tauri-apps/plugin-dialog` | Native file dialogs |
| `@tauri-apps/plugin-fs` | File system access |
| `@stegakir/aikit` | AI/LLM utilities (`ConversationalArchetype`, `Conversation`, `MemoryMessageStore`) |
| `@ai-sdk/provider` | Vercel AI SDK types (LanguageModelV3 interface) |
| `open-props` | CSS design tokens |
| `marked` | Markdown rendering |
| `zod` | Runtime validation |
| `vite` | Build tool (dev dependency) |
| `@vitejs/plugin-vue` | Vite Vue plugin (dev dependency) |
| `vue-tsc` | Vue TypeScript checking (dev dependency) |

### Backend (engine/src/Cargo.toml)

| Crate | Purpose |
|-------|---------|
| `tauri` | Desktop app framework |
| `tauri-plugin-sql` | SQLite plugin |
| `tauri-plugin-dialog` | Native dialog plugin |
| `tauri-plugin-fs` | File system plugin |
| `reqwest` | HTTP client (streaming SSE) |
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
