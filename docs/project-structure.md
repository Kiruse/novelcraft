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
│   │   └── default.json              # Tauri v2 plugin permissions (store, dialog, fs)
│   └── src/
│       ├── src/
│       │   ├── main.rs               # Entry point
│       │   ├── lib.rs                # App builder, plugin registration, command handler
│       │   ├── util/
│       │   │   ├── mod.rs            # File I/O helpers (serialize, deserialize, ensure_dir), timestamp serde, discard_channel
│       │   │   └── prompting.rs      # Prompt formatting (PromptFormatter, Promptify trait, PromptifyError)
│       │   ├── commands/
│       │   │   ├── mod.rs            # Module barrel
│       │   │   ├── llm.rs            # LLM proxy (HTTP streaming via reqwest, delegates SSE to util)
│       │   │   ├── game.rs            # Game agent loop (game_prompt, game_fork, game_sessions, game_page)
│       │   │   ├── profile.rs        # Profile persistence (OnceCell<Mutex<ProfilesFile>>)
│       │   │   ├── session.rs  # Session/page/snapshot persistence (JSON files)
│       │   │   ├── story.rs    # Story persistence (JSON files)
│       │   │   ├── lore.rs     # Lore persistence (JSON files)
│       │   │   └── fs.rs       # File operations (export/import, file dialogs)
│       │   ├── game/               # Game engine module
│       │   │   ├── mod.rs            # Module barrel
│       │   │   ├── engine.rs         # GameEngine (session loading, page CRUD, fork, history, LRU page cache via moka)
│       │   │   ├── session.rs        # Game session model (SessionV1, file-based persistence)
│       │   │   ├── pages.rs          # Page types (PageV1, PageBatch, ResponseV1)
│       │   │   ├── state.rs          # AppState (config + profiles, initialized in lib.rs setup)
│       │   │   └── module.rs         # Game module abstractions
│       │       └── infer/
│       │           ├── mod.rs            # Module barrel (pub mod api, pub mod internal)
│       │           ├── api.rs            # OpenAI API types (request/response structs for SSE)
│       │           └── internal.rs       # Command-level types (ModelConfig, LlmMessage, LlmTool, LlmPromptRequest, etc.)
│       ├── Cargo.toml                # Rust dependencies
│       ├── build.rs                  # Tauri build script
│       ├── tauri.conf.json           # Tauri configuration
│       └── icons/                    # App icons
├── gui/                              # Rust binary crate (novelcraft-gui, binary name: novelcraft) — gpui UI
│   ├── Cargo.toml              # GUI dependencies (novelcraft-engine, gpui, gpui_platform, log)
│   └── src/
│       ├── main.rs             # Entry point — gpui app initialization
│       ├── util.rs             # Loggable trait, LogLevel enum (Result<T,E> blanket impl)
│       ├── main.ts                   # [vestigial] App entry, mounts Vue + Router
│       ├── App.vue                   # [vestigial] Root component — switches between Onboarding and Main
│       ├── Main.vue                  # [vestigial] Main app shell (sidebar, router view, shortcuts, dialogs)
│       ├── Onboarding.vue            # [vestigial] Step-based first-run onboarding flow
│       ├── bindings.ts               # [vestigial] tauri-specta generated command bindings (auto-generated in debug builds)
│       ├── env.d.ts                  # [vestigial] Global type declarations for auto-imports (TS only)
│       ├── vite-env.d.ts             # [vestigial] Vite client type declarations
│   │   ├── pages/                    # Vue Router pages
│   │   │   ├── index.vue             # Home page (hero, recent vignettes, empty state)
│   │   │   ├── builder.vue           # Story builder page
│   │   │   ├── settings.vue          # Settings page — renders ModelsConfigurator component, Debug Zone (debug mode toggle)
│   │   │   └── vignettes/
│   │   │       ├── index.vue         # Vignette list (reads via commands.sessionList())
│   │   │       └── [id].vue          # Vignette play page (session/page/snapshot commands)
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
│   │   │   ├── useProfiles.ts        # Player profiles CRUD (commands.profileList() etc via unwrap)
│   │   │   ├── useLlmStream.ts       # Centralized LLM streaming client (supports tools via ToolSet)
│   │   │   ├── useGame.ts            # Stateless gameplay loop (buildMessages, buildProfileContext, returns {response, toolCalls, state} — no persistence)
│   │   │   ├── useShortcutsDialog.ts # Shared state for ShortcutsDialog (open/show/close/toggle)
│   │   │   ├── useHostLiveness.ts    # LLM host liveness checking (singleton state)
│   │   │   ├── useOnboarding.ts      # Onboarding completion state (singleton, tauri-plugin-store)
│   │   │   ├── useStoryBuilder.ts    # Story builder logic (commands.storyGet/storySave, uses createDefaultRegistry)
│   │   │   ├── useVignettes.ts       # Vignette list data & operations (singleton shared state: recent, hasMore, vignettes, create, remove, refresh)
│   │   │   ├── useVignette.ts        # Single vignette session (2-step push/fork pattern, snapshots via commands, fork with snapshot cleanup via createDefaultRegistry)
│   │   │   ├── useToast.ts           # Toast notification system
│   │   │   └── useDebugMode.ts       # Debug mode toggle (singleton, persisted to localStorage)
│   │   ├── gameplay/                 # Game modules
│   │   │   ├── index.ts              # Barrel export (createDefaultRegistry + re-exports)
│   │   │   ├── gameplayModule.ts     # Core types (GameplaySession, GameplayModule, ToolResult, GameplayModuleRegistry, createDefaultRegistry, defineGameplayModule with .withTool() builder)
│   │   │   ├── npcModule.ts          # NPC management module
│   │   │   ├── planModule.ts         # Roadmap/plan module (updateRoadmap tool, auto-prefixed to plan::updateRoadmap)
│   │   │   └── loreModule.ts         # Lore/knowledge module (query tool, auto-prefixed to lore::query, uses commands.loreQuery())
│   │   ├── utils/                    # Frontend utilities
│   │   │   ├── index.ts              # TypedResult type + unwrap helper for tauri-specta bindings
│   │   │   ├── tauriLanguageModel.ts # TauriLanguageModel (LanguageModelV3 bridge) & createTauriModel()
│   │   │   ├── msgUtils.ts           # Message utilities
│   │   │   └── suggestionParser.ts   # Suggestion parsing utilities
│   │   ├── prompts.ts                # Prompts & personas (single source of truth)
│   │   ├── router/                   # Vue Router configuration
│   │   │   └── index.ts              # Route definitions
│   │   └── assets/css/               # Open Props CSS
│   ├── vite-plugins/
│   │   └── auto-import.ts            # Custom Vite plugin: injects Vue/vue-router imports at build time
│   ├── index.html                    # Vite entry HTML
│   ├── vite.config.ts                # Vite config with ~ alias & auto-import plugin
│   ├── tsconfig.json                 # TypeScript config with ~/* paths
│   └── package.json                  # Dependencies
└── docs/                             # Project documentation
    ├── project-structure.md          # This file
    ├── code-conventions.md           # Code styling and conventions
    ├── database-schema.md            # JSON file formats and storage layout
    ├── api-routes.md                 # Tauri command documentation
    └── frontend-architecture.md      # Frontend pages, components, and composables
```

## Key Directories

### `engine/` — Rust Backend

Contains the Tauri v2 Rust backend. The actual Cargo project lives in `engine/src/`.

**`engine/src/src/`**
- `main.rs` — Binary entry point
- `lib.rs` — App builder, plugin registration (`store`, `dialog`, `fs`), command handler registration. Setup spawns async initialization of `AppState` (via `AppState::init()` — loads `NovelCraftConfig`, `Profiles`, and `Models`).
- `util/` — Engine utility modules
  - `mod.rs` — File I/O helpers: `serialize`, `deserialize`, `ensure_dir`. Timestamp serde helpers (`serialize_timestamp`, `deserialize_timestamp`). `discard_channel()` for discarding mpsc receiver items.
  - `prompting.rs` — Prompt formatting utilities for building LLM prompts with managed indentation. `PromptFormatter` buffers output and tracks `at_line_start` state; `write()` auto-indents after newlines and trims leading whitespace on continuations; `indented()` increments indent by 2 spaces for the duration of a callback and propagates errors; `as_str()` and `finish()` provide access to the buffer. `Promptify` trait defines `to_prompt(&self, f: &mut PromptFormatter) -> PromptifyResult` for types that can format themselves into a prompt. `PromptifyError` enum has variants: `Generic(String)`, `IndentOverflow { max }`, `IndentUnderflow`.
- `config.rs` — `NovelCraftConfig` struct (currently holds `max_agent_steps: u8`, default 10). Loaded from `{app_config_dir}/config.json`. Held in `AppState.config: Mutex<NovelCraftConfig>`.
- `commands/` — Tauri commands organized by domain
  - `llm.rs` — LLM proxy: builds requests, streams from OpenAI-compatible API via `reqwest`, delegates SSE frame parsing to `util::process_stream()`, emits `llm:text`/`llm:reasoning`/`llm:tool_call`/`llm:error`/`llm:done` events. Model registry (`Models` struct) is held in `AppState.models: Mutex<Models>`, initialized via `Models::load()` in `AppState::init()`. `Models` has instance methods `get_config(usage)`, `all_configs()`, `load(app)`, `save(app)`.
  - `game.rs` — Game agent loop: `game_prompt` spawns an async agent loop that reads session history via `GameEngine`, adds the prompt as a user message (if non-empty), and iterates LLM calls (up to `max_agent_steps` from `AppState.config`) until a final text response or tool call resolution. `game_fork` truncates at a page index then runs the same agent loop. Events are emitted as `gamePrompt[{stream_id}]`. Also exposes `game_sessions` (list sessions) and `game_page` (get a page).
  - `profile.rs` — Profile persistence: CRUD for profiles stored in `profiles.json`. Uses `OnceCell<Mutex<ProfilesFile>>` pattern, initialized via `init_profiles()` called in `lib.rs` setup. Active profile tracked at file level via `active_id: Option<String>` in `ProfilesFile` (not per-profile `active` field). Profile struct has `fields: serde_json::Value`.
  - `session.rs` — Session/page/snapshot persistence: CRUD for session metadata (`meta.json`), pages (`pages.{batch}.json`), and state snapshots (`state.head.json`, `state.{batch}.json`). All types include a `version` field with `read_versioned_json()` for forward-compatible deserialization.
  - `story.rs` — Story persistence: stories (`stories/{id}.json`). All types include a `version` field with version-gated deserialization.
  - `lore.rs` — Lore persistence: lore (`lore/{id}.json`). All types include a `version` field with version-gated deserialization.
  - `fs.rs` — File operations: export/import session JSON, native file/folder picker dialogs.
  - `mod.rs` — Module barrel
- `game/` — Game engine module (separate from `commands/game.rs`)
  - `engine.rs` — `GameEngine`: loads game sessions, manages page batches, provides history for prompting, page CRUD, and fork. Constructed via `GameEngine::new()`. Uses an LRU cache (`moka::future::Cache`, capacity 8) keyed by `(session_id, batch_num)` to avoid redundant disk reads when accessing pages. The `page()` method computes the batch number from the page index, checks the cache, loads from disk on miss via `PageBatchV1::load()`, and returns the specific page from the batch. `fork(page_index)` truncates page batch files after the batch containing the given index, reloads the session via `SessionV1::load` to obtain correct `tail_batches`, `page_count`, `batch_count`, and `gamestate`, and rebuilds the agent loop.
  - `session.rs` — `SessionV1`: game session model with file-based persistence (separate from `commands/session.rs`).
  - `pages.rs` — Page types: `PageV1`, `PageBatch`, `ResponseV1`.
  - `state.rs` — `AppState`: holds `config: Mutex<NovelCraftConfig>`, `profiles: Mutex<Profiles>`, and `models: Mutex<Models>`. Initialized via `AppState::init()` in `lib.rs` setup (loads config, profiles, and models).
  - `module.rs` — Game module abstractions.
  - `mod.rs` — Module barrel
- `infer/` — LLM API type definitions (extracted from `commands/llm.rs`)
  - `api.rs` — OpenAI API-related types: request structs (`ChatCompletionRequest`, `ApiChatMessage`, `ApiTool`, etc.), response structs (`StreamResponse`, `StreamChoice`, `StreamDelta`, etc.), and shared types (`FunctionCall`, `ToolCall`)
  - `internal.rs` — Command-level types: `ModelConfig`, `LlmMessage`, `LlmTool`, `LlmPromptRequest`, `LlmToolCallDelta`, `LlmDonePayload`, etc. Uses `#[specta(type = Any)]` for `serde_json::Value` fields
  - `mod.rs` — Module barrel

**`engine/capabilities/default.json`**
- Tauri v2 capability file granting plugin permissions for the main window
- Permissions: `dialog:default`, `dialog:allow-open`, `dialog:allow-save`, `dialog:allow-message`, `dialog:allow-ask`, `dialog:allow-confirm`, `fs:default`, `fs:allow-read`, `fs:allow-write`, `store:default`

**`engine/src/tauri.conf.json`**
- `frontendDist` points to `../../gui/dist`
- `devUrl` points to `http://localhost:5173`

### `gui/` — Rust GUI (gpui) + Vestigial Vue Frontend

Rust binary crate (`novelcraft-gui`, binary name `novelcraft`) using gpui for native rendering. Depends on `novelcraft-engine` as a library.

**`gui/src/` (Rust — active)**
- `main.rs` — Binary entry point, gpui app initialization
- `util.rs` — `Loggable` trait with `LogLevel` enum (`Trace`/`Debug`/`Info`/`Warn`/`Error`), `From<LogLevel> for log::Level`, and a blanket `impl<T, E: Display> Loggable for Result<T, E>` that logs `"Ok"` or `"Err: {e}"`. Convenience methods `trace()`/`debug()`/`info()`/`warn()`/`error()` delegate to the required `log(&self, level)` method.

**Note:** `gui/src/` also contains vestigial Vue 3 frontend files from the previous Tauri architecture. These are no longer used and should not be modified.

**`gui/src/` (TypeScript/Vue — vestigial)**
- `main.ts` — [vestigial] App entry, mounts Vue + Router
- `App.vue` — Root component; uses top-level await on `useOnboarding()` to switch between `Onboarding.vue` and `Main.vue`
- `Main.vue` — Main app shell: sidebar, router view, keyboard shortcuts, global dialogs
- `Onboarding.vue` — Step-based first-run onboarding (welcome, model configuration)
- `env.d.ts` — Global type declarations for auto-imported identifiers (Vue composition API, vue-router) — provides TypeScript support only; runtime injection is handled by the Vite auto-import plugin
- `prompts.ts` — Prompts and personas — single source of truth for the entire app

**`gui/src/pages/`**
- File-based routing via Vue Router (not Nuxt file-based routing)
- Dynamic routes use `[param].vue` syntax
- Vignette pages use Tauri commands for data persistence

**`gui/src/components/`**
- Reusable Vue components
- Explicit imports required: `import MyComponent from '~/components/MyComponent.vue'`
- Components using LLM streaming use `useLlmStream` composable

**`gui/src/composables/`**
- Shared reactive logic wrapped in `use*.ts` functions
- Vue composition API and vue-router functions are auto-imported at build time by the Vite auto-import plugin (`gui/vite-plugins/auto-import.ts`)
- Key composables:
  - `useLlmStream` — Centralized LLM streaming with optional `tools` support (never duplicate Tauri event listening)
   - `useGame` — Stateless gameplay loop: builds messages with module knowledge, creates registry via `createDefaultRegistry()`, builds tool set via `registry.getToolSet()` (which delegates to `registry.executeTool()` for immer state transitions), tracks tool calls. Returns `{ response, toolCalls, state }` — does NOT persist. Also contains `buildMessages()` and `buildProfileContext()` (moved from deleted `llmHelpers.ts`)
   - `useVignette` — Single vignette session management with 2-step push/fork pattern: `push()`/`fork()` return a `PromptUpdater` for consumers to finalize after `run()`. All persistence via Tauri commands. Manages snapshots, fork with snapshot cleanup via tool call replay through `registry.executeTool()` (with `init()` fallback for uninitialized module state). No direct `immer` import — delegates all state transitions to the registry. Exposes `getGameplaySession()` and readonly `snapshot` ref
   - `useVignettes` — Vignette list data and CRUD operations via `commands.sessionList/sessionCreate/sessionDelete()` (wrapped with `unwrap()`). `vignettes`, `recent`, `hasMore` are module-level singletons so all consumers share the same reactive state. Returns `{ vignettes, recent, hasMore, create, remove, refresh, loadVignettes }`
   - `useStoryBuilder` — Story builder logic, saves/loads via `commands.storySave()/storyGet()` (wrapped with `unwrap()`), uses `createDefaultRegistry()`
  - `useHostLiveness` — LLM host liveness checking with singleton state (ping_hosts via Tauri)
  - `useDebugMode` — Debug mode toggle with singleton `debugMode` ref persisted to localStorage

**`gui/src/gameplay/`**
- Game modules used by the frontend
- Core types: `GameplaySession` (simplified: `{ storyId, sessionId, state }` — no `modules` field), `GameplayModule`, `ExecuteToolResult`, `ToolResult`, `ToolCallRecord`, `GameplayModuleRegistry`, `createDefaultRegistry`, `defineGameplayModule` (with `.withTool()` builder)
- Tool execution uses `immer` for immutable state transitions (`createDraft`/`finishDraft` centralized in `GameplayModuleRegistry.executeTool()`) — both `getToolSet()` and `replay()` in `useVignette` delegate to `executeTool()`. Tools mutate `ctx.state` directly instead of using spread operators
- Registered modules: `NPCModule`, `PlanModule`, `LoreModule`
- Barrel export via `index.ts` with `createDefaultRegistry()` factory
- Consumed by `useGame`, `useStoryBuilder`, and other composables

**Module descriptions:**
- `npcModule.ts` — NPC management module
- `planModule.ts` — Roadmap/plan module; `getKnowledge()` returns `{ roadmap }`, tool `updateRoadmap` (auto-prefixed to `plan::updateRoadmap`)
- `loreModule.ts` — Lore/knowledge module; no knowledge injection, tool `query` (auto-prefixed to `lore::query`) uses `commands.loreQuery()` to search lore entries; accesses story ID via `session.storyId`

**`gui/src/utils/`**
- Pure utility functions
- `index.ts` — `TypedResult` type and `unwrap()` helper for tauri-specta bindings (converts `typedError` discriminated union to throw-on-error)
- `tauriLanguageModel.ts` — `TauriLanguageModel` class (implements `LanguageModelV3` from `@ai-sdk/provider`) and `createTauriModel()` factory function; bridges Tauri LLM proxy to Vercel AI SDK. Uses raw `invoke('prompt', ...)` (fire-and-forget)
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
- snake_case: `llm.rs`, `fs.rs`, `session.rs`, `story.rs`, `lore.rs`
- One file per domain in `engine/src/src/commands/`

## Related Documentation

- [Code Conventions](./code-conventions.md) - Styling guidelines and import patterns
- [Data Storage](./database-schema.md) - JSON file formats and storage layout
- [API Routes](./api-routes.md) - Tauri command documentation
- [Frontend Architecture](./frontend-architecture.md) - Pages, components, composables, and styling
