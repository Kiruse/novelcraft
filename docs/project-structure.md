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
│       │   └── commands/
│       │       ├── mod.rs            # Module barrel
│       │       ├── llm.rs            # LLM proxy (HTTP streaming via reqwest)
│       │       └── fs.rs             # File export/import, file dialogs
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
│   │   │   ├── settings.vue          # Settings page — renders ModelsConfigurator component
│   │   │   └── vignettes/
│   │   │       ├── index.vue         # Vignette list (reads from local SQLite)
│   │   │       └── [id].vue          # Vignette play page (local SQLite)
│   │   ├── components/               # Reusable Vue components (explicit imports)
│   │   │   ├── StoryCard.vue         # Story card component
│   │   │   ├── AppSidebar.vue        # Navigation sidebar
│   │   │   ├── AccountBox.vue        # User account box (avatar, profile, menu)
│   │   │   ├── ProfilesDialog.vue    # Profile management modal dialog
│   │   │   ├── Game.vue              # Main gameplay component
│   │   │   ├── ChatArea.vue          # Chat/conversation display
│   │   │   ├── GameDebugPanel.vue    # Gameplay debug panel
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
│   │   │   ├── useLocalDb.ts         # SQLite wrapper via tauri-plugin-sql
│   │   │   ├── useProfiles.ts        # Player profiles CRUD (local SQLite)
│   │   │   ├── useLlmStream.ts       # Centralized LLM streaming client
│   │   │   ├── useShortcutsDialog.ts # Shared state for ShortcutsDialog (open/show/close/toggle)
│   │   │   ├── useHostLiveness.ts    # LLM host liveness checking (singleton state)
│   │   │   ├── useOnboarding.ts      # Onboarding completion state (singleton, local SQLite)
│   │   │   ├── useStoryBuilder.ts    # Story builder logic
│   │   │   ├── useVignetteList.ts    # Vignette list data & operations
│   │   │   └── useToast.ts           # Toast notification system
│   │   ├── gameplay/                 # Game modules
│   │   │   ├── index.ts              # Barrel export
│   │   │   ├── gameplayModule.ts     # Core gameplay logic
│   │   │   ├── systemPromptModule.ts # System prompt handling
│   │   │   ├── eventModule.ts        # Event system
│   │   │   ├── npcModule.ts          # NPC management
│   │   │   └── graphMapModule.ts     # Graph/map module
│   │   ├── utils/                    # Frontend utilities
│   │   │   ├── llmHelpers.ts         # LLM prompt building helpers (e.g. buildProfileContext)
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
- `commands/` — Tauri commands organized by domain
  - `llm.rs` — LLM proxy: streams from OpenAI-compatible API, emits `llm:text`/`llm:reasoning`/`llm:error`/`llm:done` events
  - `fs.rs` — File operations: export/import sessions, native file/folder pickers
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
- `env.d.ts` — Global type declarations for auto-imported identifiers (`select()`, `execute()`, Vue composition API, vue-router) — provides TypeScript support only; runtime injection is handled by the Vite auto-import plugin
- `prompts.ts` — Prompts and personas — single source of truth for the entire app

**`gui/src/pages/`**
- File-based routing via Vue Router (not Nuxt file-based routing)
- Dynamic routes use `[param].vue` syntax
- Vignette pages use local SQLite via raw SQL

**`gui/src/components/`**
- Reusable Vue components
- Explicit imports required: `import MyComponent from '~/components/MyComponent.vue'`
- Components using LLM streaming use `useLlmStream` composable

**`gui/src/composables/`**
- Shared reactive logic wrapped in `use*.ts` functions
- Vue composition API and vue-router functions are auto-imported at build time by the Vite auto-import plugin (`gui/vite-plugins/auto-import.ts`)
- `select()` and `execute()` are declared in `env.d.ts` for TypeScript support
- Key composables:
  - `useLocalDb` — SQLite wrapper via tauri-plugin-sql (exports `select<T>()` and `execute()`)
  - `useLlmStream` — Centralized LLM streaming (never duplicate Tauri event listening)
  - `useHostLiveness` — LLM host liveness checking with singleton state (ping_hosts via Tauri)
  - `useStoryBuilder` — Story builder logic (imports modules from `~/gameplay`)
  - `useVignetteList` — Vignette list data and CRUD operations

**`gui/src/gameplay/`**
- Game modules used by the frontend
- Contains: `gameplayModule.ts`, `systemPromptModule.ts`, `eventModule.ts`, `npcModule.ts`, `graphMapModule.ts`
- Barrel export via `index.ts`
- Consumed by `useStoryBuilder` and other composables

**`gui/src/utils/`**
- Pure utility functions
- `llmHelpers.ts` — `buildProfileContext()` and other LLM prompt helpers
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

## File Naming Conventions

### Pages
- Pattern: `[route].vue` (e.g., `index.vue`)
- Dynamic routes: `[route]/[param].vue` (e.g., `vignettes/[id].vue`)

### Components
- PascalCase: `StoryCard.vue`, `SuggestionPicker.vue`
- Explicit imports required

### Composables
- PascalCase with `use` prefix: `useLocalDb.ts`, `useLlmStream.ts`

### Rust Commands
- snake_case: `llm.rs`, `fs.rs`
- One file per domain in `engine/src/src/commands/`

## Related Documentation

- [Code Conventions](./code-conventions.md) - Styling guidelines and import patterns
- [Database Schema](./database-schema.md) - Local SQLite table definitions
- [API Routes](./api-routes.md) - Tauri command documentation
- [Frontend Architecture](./frontend-architecture.md) - Pages, components, composables, and styling
