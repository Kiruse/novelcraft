# Agents Documentation

## Project Structure

```
novelcraft/
├── scripts/
│   ├── generate.ts                # Commander CLI program for running generators
│   ├── generators/                # Generator modules (scanned recursively)
│   │   └── better-auth/
│   │       └── schema.ts          # Drizzle schema generator for Better Auth
│   └── utils/
│       ├── get-project-root.ts    # Utility to find project root directory
│       └── index.ts               # Utilities export point
├── server/
│   ├── db/
│   │   ├── index.ts               # Database connection and db instance export
│   │   ├── schema/
│   │   │   ├── index.ts           # Schema export point
│   │   │   ├── auth.ts            # Better-Auth user tables
│   │   │   ├── app.ts             # App-specific tables (stories, sessions, etc.)
│   │   │   └── placeholder.ts     # Placeholder tables
│   │   └── migrations/            # Generated migration files
│   └── auth.ts                    # Better-Auth configuration
├── package.json                   # Project dependencies and scripts
└── README.md                      # User-facing documentation
```

## Code Styling and Conventions

### File Organization

- **Generators**: Located in `scripts/generators/` - each file exports a generator function as default
- **Utilities**: Located in `scripts/utils/` - centralized exports including helper functions
- **CLI Entry Point**: `scripts/generate.ts` - Commander-based CLI that auto-discovers generators
- **TypeScript**: All source files use `.ts` extension
- **Module type**: ESM modules (`"type": "module"` in package.json)
- **Export patterns**: Utilities use named exports; generators use default exports

### Import Patterns

Use ES module imports. Local module imports must use `.js` extension for ESM compatibility:

```typescript
import { access, constants } from "fs/promises";
import { dirname, resolve } from "path";
import { fileURLToPath } from "url";

// Local module imports from same directory
import { getProjectRoot, defineGenerator } from './utils/index.js';

// Or use relative paths from subdirectories
import { defineGenerator, getProjectRoot } from '../../utils';
```

Note: Node.js built-in modules (e.g., `fs/promises`, `path`) do not require `.js` extensions.

### Async Functions

Utility functions that perform I/O operations should be async:

```typescript
export async function getProjectRoot(startDir?: string): Promise<string> {
  // implementation
}
```

### Error Handling

Functions should throw descriptive errors:

```typescript
throw new Error("Could not find project root (no package.json found)");
```

### Path Resolution

Use Node.js path utilities for cross-platform compatibility:

- `resolve()` - resolve to absolute path
- `dirname()` - get directory name
- `fileURLToPath()` - convert import.meta.url to file path (for ES modules)

### Function Exports

Export functions explicitly using `export` keyword:

```typescript
export async function getProjectRoot(startDir?: string): Promise<string>
```

## CLI Generator System

### Overview

The project includes a CLI generator system built with Commander that automatically discovers and runs generator functions based on file hierarchy.

### Running Generators

Generators are invoked via the CLI using the command hierarchy derived from the file path:

```bash
bun run cmd:generate <generator-path>

# Example: run the Better Auth schema generator
bun run cmd:generate better-auth schema
```

### Creating a New Generator

1. Create a new `.ts` file in `scripts/generators/` with a default export:

```typescript
import { defineGenerator, getProjectRoot } from '../../utils';
import * as fs from 'fs/promises';
import * as path from 'path';

export default defineGenerator(async () => {
  const root = await getProjectRoot();
  const filepath = path.join(root, 'path/to/output.txt');
  await fs.writeFile(filepath, 'Generated content');
});
```

2. The generator will be automatically available as a subcommand based on its path:
   - `scripts/generators/mygen.ts` → `bun run cmd:generate mygen`
   - `scripts/generators/auth/schema.ts` → `bun run cmd:generate auth schema`

### defineGenerator<T>(fn): Generator

A simple wrapper function for generator functions.

**Type Parameters:**
- `T`: The return type of the generator function (optional)

**Parameters:**
- `fn`: An async function that performs the generation logic

**Returns:** The passed-through generator function

**Usage:**

```typescript
import { defineGenerator } from '../../utils';

export default defineGenerator(async () => {
  // Your generator logic here
});
```

### Generator Execution

Generators are invoked with no arguments or options. Any configuration should be handled within the generator function itself, typically by reading configuration files or environment variables.

## Base Components

### getProjectRoot(startDir?: string): Promise<string>

Utility function that traverses the file system to find the project root.

**Parameters:**
- `startDir` (optional): Starting directory path. Defaults to the directory of the calling module.

**Returns:** Absolute path to the project root directory as a Promise<string>.

**Throws:** Error if no `package.json` file is found in any parent directory.

**Usage:**

```typescript
import { getProjectRoot } from '../../utils';

// Find root from current module location
const root = await getProjectRoot();

// Find root from a specific directory
const root = await getProjectRoot('/some/path');
```

## Code Formatting

The project uses Prettier for code formatting. Prettier uses default configuration as no `.prettierrc` file exists.

Generators use prettier to format output:

```typescript
const formattedCode = await prettier.format(code, {
  parser: "typescript",
});
```

## Package Scripts

```json
{
  "scripts": {
    "build": "nuxt build",
    "dev": "nuxt dev",
    "generate": "nuxt generate",
    "preview": "nuxt preview",
    "postinstall": "nuxt prepare",
    "db:generate": "drizzle-kit generate",
    "db:migrate": "drizzle-kit migrate",
    "db:push": "drizzle-kit push",
    "db:studio": "drizzle-kit studio",
    "cmd:generate": "bun run scripts/generate.ts"
  }
}
```

## Database Schema

### Schema Organization

Database schema is organized across multiple files in `server/db/schema/`:

- **auth.ts**: Better-Auth user tables (user, session, account, etc.)
- **app.ts**: Application-specific tables (stories, game sessions, etc.)
- **placeholder.ts**: Placeholder tables for development
- **index.ts**: Central export point for all schema definitions

### Application Tables

#### stories

Stores story content and version history.

**Columns:**
- `id` (serial, PK) - Primary key
- `story_id` (text, NOT NULL) - UUID for version tracking
- `author_id` (text, FK → users.id, DELETE RESTRICT) - Story author
- `version` (integer, NOT NULL) - Story version number
- `title` (text, NOT NULL) - Story title
- `description` (text) - Story description
- `modules` (jsonb, NOT NULL) - Story module configuration
- `created_at` (timestamp, NOT NULL, default now())
- `updated_at` (timestamp, NOT NULL, auto-update)

**Indexes:**
- `stories_author_story_version_idx` on `(author_id, story_id, version)`

**Relations:**
- `author`: one-to-one with `user`
- `gameSessions`: one-to-many with `game_sessions`

#### game_sessions

Stores player game sessions.

**Columns:**
- `id` (serial, PK) - Primary key
- `player_id` (text, FK → users.id, DELETE CASCADE) - Player reference
- `story_id` (integer, FK → stories.id, DELETE CASCADE) - Story reference
- `data` (jsonb, NOT NULL) - Session game state
- `created_at` (timestamp, NOT NULL, default now())
- `updated_at` (timestamp, NOT NULL, auto-update)

**Indexes:**
- `game_sessions_player_updated_idx` on `(player_id, updated_at)`

**Relations:**
- `player`: one-to-one with `user`
- `story`: one-to-one with `stories`
- `moduleRuntime`: one-to-many with `module_runtime`
- `messages`: one-to-many with `game_session_messages`

#### module_runtime

Stores runtime state of modules.

**Columns:**
- `id` (serial, PK) - Primary key
- `game_session_id` (integer, FK → game_sessions.id, DELETE CASCADE) - Session reference
- `module_id` (text, NOT NULL) - Module identifier (not a FK)
- `data` (jsonb, NOT NULL) - Module runtime state

**Indexes:**
- `module_runtime_session_idx` on `(game_session_id)`

**Relations:**
- `gameSession`: one-to-one with `game_sessions`

#### game_session_messages

Stores conversation history for game sessions.

**Columns:**
- `id` (serial, PK) - Primary key
- `game_session_id` (integer, FK → game_sessions.id, DELETE CASCADE) - Session reference
- `role` (enum: 'system' | 'agent' | 'user' | 'toolcall', NOT NULL) - Message role
- `contents` (text, NOT NULL) - Message content
- `toolcall_data` (jsonb) - Tool call data (nullable)
- `created_at` (timestamp, NOT NULL, default now())

**Indexes:**
- `game_session_messages_session_created_idx` on `(game_session_id, created_at)`

**Relations:**
- `gameSession`: one-to-one with `game_sessions`

### Enums

#### message_role

Enum defining message roles: `'system'`, `'agent'`, `'user'`, `'toolcall'`

### Schema Relations

All foreign key relations are defined using Drizzle ORM's `relations` API for type-safe queries:

```typescript
import { db } from '~/server/db';
import { stories, gameSessions } from '~/server/db/schema';

// Query with relations
const storyWithAuthor = await db.query.stories.findFirst({
  where: eq(stories.id, 1),
  with: {
    author: true,
    gameSessions: true,
  },
});
```

### Runtime Validation with drizzle-zod

The project uses `drizzle-zod` to generate Zod validation schemas from Drizzle table definitions for runtime validation.

**Generated Insert Schemas:**

```typescript
import {
  insertStorySchema,
  insertGameSessionSchema,
  insertModuleRuntimeSchema,
  insertGameSessionMessageSchema,
} from '~/server/db/schema/app';
import { z } from 'zod';

// Validate story data before insertion
const storyData = {
  storyId: 'uuid-123',
  authorId: 'user-uuid',
  version: 1,
  title: 'My Story',
  description: 'A description',
  modules: { modules: [] },
};

const validated = insertStorySchema.parse(storyData);
await db.insert(stories).values(validated);
```

**Custom Validation:**

You can extend generated schemas with custom validation rules:

```typescript
import { insertStorySchema } from '~/server/db/schema/app';
import { z } from 'zod';

const customStorySchema = insertStorySchema.extend({
  title: z.string().min(1).max(200),
  version: z.number().int().positive(),
});

const validated = customStorySchema.parse(data);
```

**API Route Validation Example:**

```typescript
export default defineEventHandler(async (event) => {
  const body = await readBody(event);
  const validated = insertGameSessionSchema.parse(body);
  const session = await db.insert(gameSessions).values(validated).returning();
  return session[0];
});
```
