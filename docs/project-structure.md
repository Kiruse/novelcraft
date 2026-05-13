# Project Structure

This document outlines the complete folder organization and directory layout for Novelcraft.

## Directory Tree

```
novelcraft/
├── justfile                          # Build orchestration (just commands)
├── AGENTS.md                         # High-level overview for AI agents
├── README.md                         # User-facing documentation
├── engine/                           # Rust backend (Tauri v2)
│   ├── capabilities/
│   │   └── default.json              # Tauri v2 plugin permissions (sql, dialog, fs)
│   └── src/
│       ├── src/
│       │   ├── main.rs               # Entry point
│       │   ├── lib.rs                # App builder, plugin registration, command handler
│       │   ├── util.rs               # SSE stream parsing (StreamEvent enum, process_stream())
│       │   ├── commands/
│       │   │   ├── mod.rs            # Module barrel
│       │   │   ├── llm.rs            # LLM proxy (HTTP streaming via reqwest, delegates SSE to util)
│       │   │   └── fs.rs             # File export/import, file dialogs
│       │   └── infer/
│       │       ├── mod.rs            # Module barrel (pub mod api)
│       │       └── api.rs            # OpenAI API types (request/response structs for SSE)
│       ├── Cargo.toml                # Rust dependencies
│       ├── build.rs                  # Tauri build script
│       ├── tauri.conf.json           # Tauri configuration
│       └── icons/                    # App icons
├── gui/                              # Vue 3 frontend (Vite + Vue Router)
│   ├── src/
│   │   ├── main.ts                   # App entry, mounts Vue + Router
│   │   ├── App.vue                   # Root component — switches between Onboarding and Main
│   │   ├── Main.vue                  # Main app shell (sidebar, router view, shortcuts, dialogs)
│   │   ├── Onboarding.vue            # Step-based first-run onboarding flow
│   │   ├── env.d.ts                  # Global type declarations for auto-imports (TS only)
│   │   ├── vite-env.d.ts             # Vite client type declarations
│   │   ├── pages/                    # Vue Router pages
│   │   │   ├── index.vue             # Home page (hero, recent vignettes, empty state)
│   │   │   ├── builder.vue           # Story builder page
│   │   │   ├── settings.vue          # Settings page — renders ModelsConfigurator component, Debug Zone (debug mode toggle)
│   │   │   └── vignettes/
│   │   │       ├── index.vue         # Vignette list (reads from local SQLite)
│   │   │       └── [id].vue          # Vignette play page (local SQLite)
│   │   ├── components/               # Reusable Vue components (explicit imports)
│   │   │   ├── StoryCard.vue         # Story card component
│   │   │   ├── AppSidebar.vue        # Navigation sidebar
│   │   │   ├── AccountBox.vue        # User account box (avatar, profile, menu)
│   │   │   ├── ProfilesDialog.vue    # Profile management modal dialog
│   │   │   ├── Game.vue              # Main gameplay component (disposition page 0, story pages, chat input, debug panel)
│   │   │   ├── ChatArea.vue          # Chat/conversation display
│   │   │   ├── GameDebugPanel.vue    # Gameplay debug panel (module runtime state)
│   │   │   ├── DebugPanel.vue        # LLM thoughts & token usage debug panel
│   │   │   ├── ShortcutRow.vue       # Keyboard shortcut row (label + kbd keys, optional highlight)
│   │   │   ├── ShortcutsDialog.vue   # Keyboard shortcuts modal dialog (search, filter)
│   │   │   ├── HostLivenessDialog.vue# LLM host liveness check modal (auto-checks on mount)
│   │   │   ├── SuggestionPicker.vue  # AI suggestion picker (uses useLlmStream)
│   │   │   ├── GameSessionCard.vue   # Vignette/session card
│   │   │   ├── MobileHeader.vue      # Mobile navigation header
│   │   │   ├── ToastContainer.vue    # Toast notification container
│   │   │   ├── ToastItem.vue         # Individual toast notification
│   │   │   ├── Chevron.vue           # Chevron icon component
│   │   │   ├── Collapsible.vue       # Collapsible wrapper component
│   │   │   ├── Spinner.vue           # Reusable CSS spinner (sm/md, accessible label)
│   │   │   ├── ModelConfigBox.vue    # LLM model config card (edit, save, cancel, debounced ping)
│   │   │   ├── ModelsConfigurator.vue # Models list section (ModelConfigBox per model, save logic)
│   │   │   ├── Tooltip.vue           # Floating tooltip (Floating UI dom, Teleport to body)
│   │   │   └── builder/
│   │   │       └── InspireDialog.vue # Inspiration dialog (uses useLlmStream)
│   │   ├── composables/              # Vue composables
│   │   │   ├── useProfiles.ts        # Player profiles CRUD (Drizzle ORM)
│   │   │   ├── useLlmStream.ts       # Centralized LLM streaming client (supports tools via ToolSet)
│   │   │   ├── useGame.ts            # Stateless gameplay loop (buildMessages, buildProfileContext, returns {response, toolCalls, state} — no persistence)
│   │   │   ├── useShortcutsDialog.ts # Shared state for ShortcutsDialog (open/show/close/toggle)
│   │   │   ├── useHostLiveness.ts    # LLM host liveness checking (singleton state)
│   │   │   ├── useOnboarding.ts      # Onboarding completion state (singleton, local SQLite)
│   │   │   ├── useStoryBuilder.ts    # Story builder logic (saves to local_stories, uses createDefaultRegistry)
│   │   │   ├── useVignettes.ts       # Vignette list data & operations (recent, hasMore, create, remove, refresh)
│   │   │   ├── useVignette.ts        # Single vignette session (2-step push/fork pattern, snapshots, fork with snapshot cleanup via createDefaultRegistry)
│   │   │   ├── useToast.ts           # Toast notification system
│   │   │   └── useDebugMode.ts       # Debug mode toggle (singleton, persisted to localStorage)
│   │   ├── gameplay/                 # Game modules
│   │   │   ├── index.ts              # Barrel export (createDefaultRegistry + re-exports)
│   │   │   ├── gameplayModule.ts     # Core types (GameplaySession, GameplayModule, ToolResult, GameplayModuleRegistry, createDefaultRegistry, defineGameplayModule with .withTool() builder)
│   │   │   ├── npcModule.ts          # NPC management module
│   │   │   ├── planModule.ts         # Roadmap/plan module (updateRoadmap tool, auto-prefixed to plan::updateRoadmap)
│   │   │   └── loreModule.ts         # Lore/knowledge module (query tool, auto-prefixed to lore::query, queries local_lore_entries)
│   │   ├── utils/                    # Frontend utilities
│   │   │   ├── tauriLanguageModel.ts # TauriLanguageModel (LanguageModelV3 bridge) & createTauriModel()
│   │   │   ├── msgUtils.ts           # Message utilities
│   │   │   └── suggestionParser.ts   # Suggestion parsing utilities
│   │   ├── db/                       # Drizzle ORM database layer
│   │   │   ├── schema.ts             # Drizzle table definitions (7 SQLite tables)
│   │   │   └── index.ts              # Drizzle instance (sqlite-proxy adapter) + table re-exports
│   │   ├── prompts.ts                # Prompts & personas (single source of truth)
│   │   ├── router/                   # Vue Router configuration
│   │   │   └── index.ts              # Route definitions
│   │   └── assets/css/               # Open Props CSS
│   ├── drizzle.config.ts            # Drizzle Kit configuration (schema path, output dir, dialect)
│   ├── drizzle/                     # Generated migration SQL files
│   │   ├── 0000_legal_venom.sql     # Initial migration (creates all 7 tables)
│   │   └── meta/
│   │       └── _journal.json        # Drizzle Kit migration journal
│   ├── vite-plugins/
│   │   └── auto-import.ts            # Custom Vite plugin: injects Vue/vue-router imports at build time
│   ├── index.html                    # Vite entry HTML
│   ├── vite.config.ts                # Vite config with ~ alias & auto-import plugin
│   ├── tsconfig.json                 # TypeScript config with ~/* paths
│   └── package.json                  # Dependencies
└── docs/                             # Project documentation
    ├── project-structure.md          # This file
    ├── code-conventions.md           # Code styling and conventions
    ├── database-schema.md            # Local SQLite table definitions
    ├── api-routes.md                 # Tauri command documentation
    └── frontend-architecture.md      # Frontend pages, components, and composables
```

## Key Directories

### `engine/` — Rust Backend

Contains the Tauri v2 Rust backend. The actual Cargo project lives in `engine/src/`.

**`engine/src/src/`**
- `main.rs` — Binary entry point
- `lib.rs` — App builder, plugin registration (`sql`, `dialog`, `fs`), command handler registration
- `util.rs` — SSE stream parsing utilities; defines `StreamEvent` enum (`Text`, `Reasoning`, `ToolCall`, `Done`) and `process_stream()` async function that parses byte streams and invokes a callback for each event
- `commands/` — Tauri commands organized by domain
  - `llm.rs` — LLM proxy: builds requests, streams from OpenAI-compatible API via `reqwest`, delegates SSE frame parsing to `util::process_stream()`, emits `llm:text`/`llm:reasoning`/`llm:tool_call`/`llm:error`/`llm:done` events
  - `fs.rs` — File operations: export/import sessions, native file/folder pickers
  - `mod.rs` — Module barrel
- `infer/` — LLM API type definitions (extracted from `commands/llm.rs`)
  - `api.rs` — OpenAI API-related types: request structs (`ChatCompletionRequest`, `ApiChatMessage`, `ApiTool`, etc.), response structs (`StreamResponse`, `StreamChoice`, `StreamDelta`, etc.), and shared types (`FunctionCall`, `ToolCall`)
  - `mod.rs` — Module barrel

**`engine/capabilities/default.json`**
- Tauri v2 capability file granting plugin permissions for the main window
- Permissions: `sql:default`, `sql:allow-load`, `sql:allow-execute`, `sql:allow-select`, `dialog:default`, `dialog:allow-open`, `dialog:allow-save`, `dialog:allow-message`, `dialog:allow-ask`, `dialog:allow-confirm`, `fs:default`, `fs:allow-read`, `fs:allow-write`

**`engine/src/tauri.conf.json`**
- `frontendDist` points to `../../gui/dist`
- `devUrl` points to `http://localhost:5173`
- SQLite preload configured for `sqlite:novelcraft.db`

### `gui/` — Vue 3 Frontend

Contains the Vue 3 + Vite frontend application. No server — this runs in a Tauri webview.

**`gui/src/`**
- `main.ts` — App entry, mounts Vue + Router
- `App.vue` — Root component; uses top-level await on `useOnboarding()` to switch between `Onboarding.vue` and `Main.vue`
- `Main.vue` — Main app shell: sidebar, router view, keyboard shortcuts, global dialogs
- `Onboarding.vue` — Step-based first-run onboarding (welcome, model configuration)
- `env.d.ts` — Global type declarations for auto-imported identifiers (Vue composition API, vue-router) — provides TypeScript support only; runtime injection is handled by the Vite auto-import plugin
- `prompts.ts` — Prompts and personas — single source of truth for the entire app

**`gui/src/pages/`**
- File-based routing via Vue Router (not Nuxt file-based routing)
- Dynamic routes use `[param].vue` syntax
- Vignette pages use local SQLite via Drizzle ORM

**`gui/src/components/`**
- Reusable Vue components
- Explicit imports required: `import MyComponent from '~/components/MyComponent.vue'`
- Components using LLM streaming use `useLlmStream` composable

**`gui/src/composables/`**
- Shared reactive logic wrapped in `use*.ts` functions
- Vue composition API and vue-router functions are auto-imported at build time by the Vite auto-import plugin (`gui/vite-plugins/auto-import.ts`)
- Key composables:
  - `useLlmStream` — Centralized LLM streaming with optional `tools` support (never duplicate Tauri event listening)
  - `useGame` — Stateless gameplay loop: builds messages with module knowledge, creates registry via `createDefaultRegistry()`, builds tool set via `registry.getToolSet()` (immer `createDraft`/`finishDraft` for state transitions), tracks tool calls. Returns `{ response, toolCalls, state }` — does NOT persist. Also contains `buildMessages()` and `buildProfileContext()` (moved from deleted `llmHelpers.ts`)
  - `useVignette` — Single vignette session management with 2-step push/fork pattern: `push()`/`fork()` return a `PromptUpdater` for consumers to finalize after `run()`. Manages snapshots, fork with snapshot cleanup via tool call replay. Exposes `getGameplaySession()` and readonly `snapshot` ref
  - `useVignettes` — Vignette list data and CRUD operations (creates snapshot 0 on creation, deletes pages + snapshots + session on removal). Returns `{ vignettes, recent, hasMore, create, remove, refresh, loadVignettes }`
  - `useStoryBuilder` — Story builder logic, saves to `local_stories` table, uses `createDefaultRegistry()`
  - `useHostLiveness` — LLM host liveness checking with singleton state (ping_hosts via Tauri)
  - `useDebugMode` — Debug mode toggle with singleton `debugMode` ref persisted to localStorage

**`gui/src/gameplay/`**
- Game modules used by the frontend
- Core types: `GameplaySession` (simplified: `{ storyId, sessionId, state }` — no `modules` field), `GameplayModule`, `ToolResult`, `ToolCallRecord`, `GameplayModuleRegistry`, `createDefaultRegistry`, `defineGameplayModule` (with `.withTool()` builder)
- Tool execution uses `immer` for immutable state transitions (`createDraft`/`finishDraft` in `GameplayModuleRegistry.getToolSet()`) — tools mutate `ctx.state` directly instead of using spread operators
- Registered modules: `NPCModule`, `PlanModule`, `LoreModule`
- Barrel export via `index.ts` with `createDefaultRegistry()` factory
- Consumed by `useGame`, `useStoryBuilder`, and other composables

**Module descriptions:**
- `npcModule.ts` — NPC management module
- `planModule.ts` — Roadmap/plan module; `getKnowledge()` returns `{ roadmap }`, tool `updateRoadmap` (auto-prefixed to `plan::updateRoadmap`)
- `loreModule.ts` — Lore/knowledge module; no knowledge injection, tool `query` (auto-prefixed to `lore::query`) queries `local_lore_entries` via Drizzle ORM LIKE search; accesses story ID via `session.storyId`

**`gui/src/db/`**
- Drizzle ORM database layer for all SQLite access
- `schema.ts` — Drizzle table definitions for all 7 SQLite tables using `sqliteTable()` from `drizzle-orm/sqlite-core`. Column property names are camelCase mapping to snake_case SQL columns.
- `index.ts` — Drizzle instance using `drizzle-orm/sqlite-proxy` adapter bridged to `@tauri-apps/plugin-sql`. Exports `db` (the Drizzle instance), `schema`, and re-exports all individual table objects. Runs drizzle-kit migrations on first connection (lazy singleton), with baseline detection for pre-migration databases.
- **Import pattern**: `import { db, localProfiles } from '~/db';` plus operators from `drizzle-orm`

**`gui/drizzle/`**
- Generated migration SQL files managed by Drizzle Kit
- `0000_legal_venom.sql` — Initial migration creating all 7 tables
- `meta/_journal.json` — Drizzle Kit migration journal (tracks order and checksums)
- Migrations are bundled at build time via `import.meta.glob` and applied by `runMigrations()` in `gui/src/db/index.ts`
- Generate new migrations with `just generate-migration`

**`gui/drizzle.config.ts`**
- Drizzle Kit configuration: `defineConfig({ schema: './src/db/schema.ts', out: './drizzle', dialect: 'sqlite' })`

**`gui/src/utils/`**
- Pure utility functions
- `tauriLanguageModel.ts` — `TauriLanguageModel` class (implements `LanguageModelV3` from `@ai-sdk/provider`) and `createTauriModel()` factory function; bridges Tauri LLM proxy to Vercel AI SDK
- `msgUtils.ts` — Message formatting and transformation
- `suggestionParser.ts` — AI suggestion parsing

**`gui/src/router/`**
- `index.ts` — Vue Router route definitions using `createRouter()`

**`gui/src/assets/css/`**
- Global stylesheets importing Open Props tokens and normalize reset

### `docs/` — Documentation

Comprehensive project documentation for developers and contributors.

## Build Orchestration

The root `justfile` is the single source of truth for all build commands. There is no top-level `package.json`.

| Recipe | Purpose |
|--------|---------|
| `just dev-frontend` | Start Vite dev server in `gui/` |
| `just dev-engine` | Start `cargo tauri dev` in `engine/src/` |
| `just build-frontend` | Vite production build |
| `just build-engine` | cargo tauri build |
| `just build` | Both frontend + engine |
| `just typecheck` | `vue-tsc --noEmit` in `gui/` |
| `just check` | typecheck + `cargo check` |
| `just clippy` | cargo clippy in `engine/` |
| `just fmt` | cargo fmt in `engine/` |
| `just fmt-check` | cargo fmt --check in `engine/` |
| `just generate-migration` | Generate Drizzle Kit migration from schema changes (`cd gui && bunx drizzle-kit generate`) |

## File Naming Conventions

### Pages
- Pattern: `[route].vue` (e.g., `index.vue`)
- Dynamic routes: `[route]/[param].vue` (e.g., `vignettes/[id].vue`)

### Components
- PascalCase: `StoryCard.vue`, `SuggestionPicker.vue`
- Explicit imports required

### Composables
- PascalCase with `use` prefix: `useLlmStream.ts`, `useProfiles.ts`

### Rust Commands
- snake_case: `llm.rs`, `fs.rs`
- One file per domain in `engine/src/src/commands/`

## Related Documentation

- [Code Conventions](./code-conventions.md) - Styling guidelines and import patterns
- [Database Schema](./database-schema.md) - Local SQLite table definitions
- [API Routes](./api-routes.md) - Tauri command documentation
- [Frontend Architecture](./frontend-architecture.md) - Pages, components, composables, and styling
