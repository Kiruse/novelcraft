# Project Structure

This document outlines the complete folder organization and directory layout for Novelcraft.

## Directory Tree

```
novelcraft/
├── app/                          # Frontend application (Nuxt 4)
│   ├── app.vue                   # Root Vue component with NuxtPage
│   ├── pages/                    # File-based routing
│   │   ├── index.vue             # Discovery/home page
│   │   └── stories/
│   │       └── [id].vue          # Story detail page (dynamic route)
│   ├── components/               # Reusable Vue components (auto-imported)
│   │   ├── StoryCard.vue         # Story card component for display
│   │   └── GameSessionCard.vue   # Game session card component
│   └── assets/
│       └── css/
│           └── app.css           # Global CSS reset and base styles
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
│   │   ├── sessions.get.ts       # GET /api/sessions - Get current user's sessions
│   │   └── stories/
│   │       └── [id].get.ts       # GET /api/stories/:id - Get story by ID
│   ├── db/                       # Database configuration and schema
│   │   ├── index.ts              # Database connection and db instance export
│   │   ├── schema/
│   │   │   ├── index.ts          # Schema export point
│   │   │   ├── auth.ts           # Better-Auth user tables
│   │   │   ├── app.ts            # App-specific tables (stories, sessions, etc.)
│   │   │   └── placeholder.ts    # Placeholder tables
│   │   └── migrations/           # Generated migration files
│   └── auth.ts                   # Better-Auth configuration
├── docs/                         # Project documentation
│   ├── project-structure.md     # This file
│   ├── code-conventions.md       # Code styling and conventions
│   ├── generator-system.md       # CLI generator system documentation
│   ├── database-schema.md        # Database tables and relations
│   ├── api-routes.md             # API endpoints documentation
│   └── frontend-architecture.md  # Frontend pages and components
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

**`pages/`**
- File-based routing system
- Each `.vue` file becomes a route
- Dynamic routes use `[param].vue` syntax

**`components/`**
- Reusable Vue components
- Auto-imported throughout the application
- No manual imports required

**`assets/css/`**
- Global stylesheets
- `app.css` includes CSS reset and base styles

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

**`db/`**
- Database connection setup
- Drizzle ORM schema definitions
- Migration files

**`schema/`**
- `auth.ts` - Better-Auth tables (user, session, account)
- `app.ts` - Application tables (stories, game_sessions, etc.)
- `placeholder.ts` - Development placeholder tables
- `index.ts` - Central schema export

### `docs/` - Documentation

Comprehensive project documentation for developers and contributors.

## File Naming Conventions

### API Routes
- Pattern: `[resource].[method].ts` (e.g., `stories.get.ts`)
- Dynamic routes: `[resource]/[param].[method].ts` (e.g., `stories/[id].get.ts`)

### Pages
- Pattern: `[route].vue` (e.g., `index.vue`)
- Dynamic routes: `[route]/[param].vue` (e.g., `stories/[id].vue`)

### Components
- PascalCase: `StoryCard.vue`, `GameSessionCard.vue`
- Auto-imported, no manual imports needed

### Generators
- File hierarchy maps to CLI commands
- Each file exports a default generator function

## Related Documentation

- [Code Conventions](./code-conventions.md) - Styling guidelines and import patterns
- [Generator System](./generator-system.md) - CLI tool usage and generator creation
- [Database Schema](./database-schema.md) - Table definitions and relations
- [API Routes](./api-routes.md) - Endpoint documentation and patterns
- [Frontend Architecture](./frontend-architecture.md) - Pages, components, and styling
