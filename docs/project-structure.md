# Project Structure

This document outlines the complete folder organization and directory layout for Novelcraft.

## Directory Tree

```
novelcraft/
├── app/                          # Frontend application (Nuxt 4)
│   ├── app.vue                   # Root Vue component with NuxtPage
│   ├── pages/                    # File-based routing
│   │   ├── index.vue             # Discovery/home page
│   │   ├── builder.vue           # Story builder page
│   │   ├── settings.vue          # Settings page
│   │   ├── stories/              # Published story pages
│   │   │   └── [author]/
│   │   │       └── [id].vue      # Story play page
│   │   ├── vignettes/            # Client-side vignette pages
│   │   │   ├── index.vue         # Vignette list (reads from local SQLite)
│   │   │   └── [id].vue          # Vignette play page (local SQLite)
│   │   ├── auth/                 # Auth pages
│   │   └── builder/              # Story builder sub-pages
│   ├── components/               # Reusable Vue components (auto-imported)
│   │   ├── StoryCard.vue         # Story card component
│   │   ├── AppSidebar.vue        # Navigation sidebar
│   │   ├── AccountBox.vue        # User account box (avatar, profile, menu)
│   │   ├── ProfilesDialog.vue    # Profile management modal dialog
│   │   ├── Game.vue              # Main gameplay component
│   │   ├── ChatArea.vue          # Chat/conversation display
│   │   ├── GameDebugPanel.vue    # Gameplay debug panel
│   │   ├── ShortcutRow.vue       # Keyboard shortcut row (label + kbd keys, optional highlight)
│   │   ├── ShortcutsDialog.vue   # Keyboard shortcuts modal dialog (search, filter)
│   │   ├── SuggestionPicker.vue  # AI suggestion picker (uses useLlmStream)
│   │   ├── builder/              # Story builder components
│   │   │   └── InspireDialog.vue # Inspiration dialog (uses useLlmStream)
│   │   └── ...                   # Other UI components
│   ├── composables/              # Vue composables (auto-imported)
│   │   ├── useLocalDb.ts         # PowerSync + Drizzle SQLite wrapper
│   │   ├── useProfiles.ts        # Player profiles CRUD (local SQLite)
│   │   ├── useLlmStream.ts       # Centralized SSE streaming client
│   │   ├── useShortcutsDialog.ts # Shared state for ShortcutsDialog (open/show/close/toggle)
│   │   ├── useStoryBuilder.ts    # Story builder logic
│   │   ├── useCurrentUser.ts     # Current user state
│   │   ├── useAuthClient.ts      # Auth client wrapper
│   │   └── useToast.ts           # Toast notification system
│   ├── plugins/                  # Nuxt plugins
│   │   └── powersync.client.ts   # PowerSync initialization (client-only)
│   └── assets/
│       └── css/
│           └── app.css           # Global CSS (Open Props + normalize)
├── shared/                       # Code shared between app and server
│   ├── gameplay/                 # Game modules (moved from server/gameplay/)
│   │   ├── index.ts              # Barrel export
│   │   ├── gameplayModule.ts     # Core gameplay logic
│   │   ├── systemPromptModule.ts # System prompt handling
│   │   ├── eventModule.ts        # Event system
│   │   ├── npcModule.ts          # NPC management
│   │   └── graphMapModule.ts     # Graph/map module
│   ├── db/                       # Client-side SQLite schema
│   │   ├── index.ts              # Barrel export
│   │   └── localSchema.ts        # Drizzle SQLite tables
│   ├── dsl/                      # Domain-specific language definitions
│   └── prompts.ts                # Prompts & personas (single source of truth)
├── scripts/                      # CLI tools and utilities
│   ├── generate.ts               # Commander CLI program for running generators
│   ├── generators/               # Generator modules (scanned recursively)
│   │   └── better-auth/
│   │       └── schema.ts         # Drizzle schema generator for Better Auth
│   └── utils/                    # Utility functions
│       ├── get-project-root.ts   # Utility to find project root directory
│       └── index.ts              # Utilities export point
├── server/                       # Backend API and database
│   ├── api/                      # API routes (file-based routing)
│   │   ├── stories.get.ts        # GET /api/stories - List all stories
│   │   ├── stories.post.ts       # POST /api/stories - Create story
│   │   ├── stories/
│   │   │   ├── [author]/
│   │   │   │   └── [id].get.ts   # GET /api/stories/:author/:id
│   │   │   ├── draft.put.ts      # PUT /api/stories/draft
│   │   │   └── publish.post.ts   # POST /api/stories/publish
│   │   ├── llm/
│   │   │   └── prompt.post.ts    # POST /api/llm/prompt - LLM proxy (SSE)
│   │   ├── auth/
│   │   │   └── [...all].ts       # Better-Auth catch-all handler
│   │   └── user/
│   │       ├── me.get.ts         # GET /api/user/me
│   │       ├── redeem-author.post.ts
│   │       └── stories.get.ts    # GET /api/user/stories
│   ├── ai/
│   │   └── models.ts             # Model registry & resolveModel()
│   ├── db/                       # Database configuration and schema
│   │   ├── index.ts              # Database connection and db instance export
│   │   ├── schema/
│   │   │   ├── index.ts          # Schema export point
│   │   │   ├── auth.ts           # Better-Auth user tables
│   │   │   ├── app.ts            # App-specific tables (stories)
│   │   │   └── placeholder.ts    # Placeholder tables
│   │   └── migrations/           # Generated migration files
│   └── auth.ts                   # Better-Auth configuration
├── docs/                         # Project documentation
│   ├── project-structure.md      # This file
│   ├── code-conventions.md       # Code styling and conventions
│   ├── database-schema.md        # Database tables and relations (server + local)
│   ├── api-routes.md             # API endpoints documentation
│   └── frontend-architecture.md  # Frontend pages, components, and composables
├── nuxt.config.ts                # Nuxt configuration
├── package.json                  # Project dependencies and scripts
├── README.md                     # User-facing documentation
└── AGENTS.md                     # High-level overview for AI agents
```

## Key Directories

### `app/` - Frontend Application

Contains the Nuxt 4 frontend application built with Vue 3.

**`app.vue`**
- Root Vue component
- Contains `<NuxtPage>` for page rendering
- Mounts `<ShortcutsDialog />` component
- Registers global `Ctrl+Shift+/` keydown listener that toggles the shortcuts dialog via `useShortcutsDialog().toggle()`

**`pages/`**
- File-based routing system
- Each `.vue` file becomes a route
- Dynamic routes use `[param].vue` syntax
- Vignette pages use local SQLite instead of server API

**`components/`**
- Reusable Vue components
- Auto-imported throughout the application
- No manual imports required
- Components using LLM streaming use `useLlmStream` composable

**`composables/`**
- Shared reactive logic wrapped in `use*.ts` functions
- Auto-imported throughout the application
- Key composables:
  - `useLocalDb` — PowerSync + Drizzle SQLite wrapper
  - `useLlmStream` — Centralized SSE streaming (never duplicate SSE parsing)
  - `useStoryBuilder` — Story builder logic (imports modules from `#shared/gameplay`)

**`plugins/`**
- Nuxt plugins
- `powersync.client.ts` — Client-only plugin that initializes PowerSync for local SQLite

**`assets/css/`**
- Global stylesheets
- `app.css` includes Open Props tokens and normalize reset

### `shared/` - Shared Code

Code shared between the frontend app and server.

**`gameplay/`**
- Game modules moved from `server/gameplay/`
- Contains: `gameplayModule.ts`, `systemPromptModule.ts`, `eventModule.ts`, `npcModule.ts`, `graphMapModule.ts`
- Barrel export via `index.ts`
- Consumed by frontend composables (e.g., `useStoryBuilder` uses `getAllModules()`)

**`db/`**
- Client-side SQLite schema for PowerSync
- `localSchema.ts` defines: `local_sessions`, `local_pages`, `local_module_runtime`, `local_profiles`
- Barrel export via `index.ts`
- Consumed by `useLocalDb` composable

**`prompts.ts`**
- Prompts and personas — single source of truth
- Both frontend and server import from here

### `scripts/` - CLI Tools

Contains build tools, generators, and utility scripts.

**`generate.ts`**
- Entry point for CLI generator system
- Built with Commander
- Auto-discovers generators in `scripts/generators/`

**`generators/`**
- Modular generator functions
- Each file exports a generator as default
- File hierarchy determines CLI command structure

**`utils/`**
- Shared utility functions
- Centralized exports via `index.ts`

### `server/` - Backend

Contains the server-side code, API routes, and database configuration.

**`api/`**
- File-based API routing
- Pattern: `[resource]/[method].ts`
- Dynamic routes use `[param].ts`
- **Pure CRUD only** — auth, user data, shareable story metadata
- No gameplay or session endpoints (those are client-side now)
- `POST /api/llm/prompt` is the sole AI endpoint (LLM proxy)

**`ai/`**
- `models.ts` — Model registry with `resolveModel(name)` function
- All models use `@ai-sdk/openai-compatible`

**`db/`**
- Database connection setup
- Drizzle ORM schema definitions
- Migration files
- Server schema is minimal: auth tables + stories only

**`schema/`**
- `auth.ts` - Better-Auth tables (user, session, account)
- `app.ts` - Application tables (stories only; gameplay tables removed)
- `placeholder.ts` - Development placeholder tables
- `index.ts` - Central schema export

### `docs/` - Documentation

Comprehensive project documentation for developers and contributors.

## File Naming Conventions

### API Routes
- Pattern: `[resource].[method].ts` (e.g., `stories.get.ts`)
- Dynamic routes: `[resource]/[param].[method].ts` (e.g., `stories/[author]/[id].get.ts`)

### Pages
- Pattern: `[route].vue` (e.g., `index.vue`)
- Dynamic routes: `[route]/[param].vue` (e.g., `vignettes/[id].vue`)

### Components
- PascalCase: `StoryCard.vue`, `SuggestionPicker.vue`
- Auto-imported, no manual imports needed

### Composables
- PascalCase with `use` prefix: `useLocalDb.ts`, `useLlmStream.ts`
- Auto-imported, no manual imports needed

### Generators
- File hierarchy maps to CLI commands
- Each file exports a default generator function

## Related Documentation

- [Code Conventions](./code-conventions.md) - Styling guidelines and import patterns
- [Database Schema](./database-schema.md) - Table definitions and relations (server + local)
- [API Routes](./api-routes.md) - Endpoint documentation and patterns
- [Frontend Architecture](./frontend-architecture.md) - Pages, components, composables, and styling
