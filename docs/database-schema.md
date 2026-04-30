# Database Schema

This document describes the complete database schema, including all tables, columns, indexes, relations, and runtime validation using drizzle-zod.

## Overview

The project has two separate databases:

1. **Server database** (PostgreSQL) — shareable entities: users, auth, stories
2. **Client-side local database** (SQLite via PowerSync) — gameplay state: vignettes, sessions, module runtime, profiles

### Server Schema

Located in `server/db/schema/`:

- **auth.ts** - Better-Auth user tables (user, session, account, etc.)
- **app.ts** - Application-specific tables (stories only)
- **index.ts** - Central export point for all schema definitions

### Local Schema

Located in `shared/db/`:

- **localSchema.ts** - Client-side SQLite tables (local_sessions, local_pages, local_module_runtime, local_profiles)
- **index.ts** - Barrel export

### Technology Stack

| Layer | ORM | Database | Validation |
|-------|-----|----------|------------|
| Server | Drizzle ORM | PostgreSQL | drizzle-zod |
| Client | Drizzle ORM via PowerSync driver | SQLite (WASM) | TypeScript types |

---

## Server Schema (PostgreSQL)

### Import Structure

```typescript
// server/db/schema/index.ts
export * from './auth';
export * from './app';
```

```typescript
// server/db/index.ts
import { drizzle } from 'drizzle-orm/node-postgres';
import pg from 'pg';
import * as schema from './schema';

const { Pool } = pg;
const pool = new Pool({ connectionString: process.env.DATABASE_URL });

export const db = drizzle(pool, { schema });
```

## Application Tables

### stories

Stores story content and version history.

**Location:** `server/db/schema/app.ts`

**Schema Definition:**

```typescript
export const stories = pgTable(
  "stories",
  {
    id: serial("id").primaryKey(),
    storyId: text("story_id").notNull(),
    authorId: text("author_id")
      .notNull()
      .references(() => user.id, { onDelete: "restrict" }),
    version: integer("version").notNull(),
    title: text("title").notNull(),
    description: text("description"),
    coverArt: text("cover_art"),
    genre: text("genre"),
    modules: jsonb("modules").notNull().$type<Record<string, unknown>>(),
    createdAt: timestamp("created_at").defaultNow().notNull(),
    updatedAt: timestamp("updated_at")
      .defaultNow()
      .$onUpdate(() => new Date())
      .notNull(),
  },
  (table) => [uniqueIndex("stories_author_story_version_idx").on(table.authorId, table.storyId, table.version)],
);
```

**Columns:**

| Column | Type | Constraints | Description |
|--------|------|-------------|-------------|
| `id` | serial | PRIMARY KEY | Auto-incrementing primary key |
| `story_id` | text | NOT NULL | UUID for version tracking across multiple story versions |
| `author_id` | text | NOT NULL, FK → users.id | Story author reference (DELETE RESTRICT) |
| `version` | integer | NOT NULL | Story version number |
| `title` | text | NOT NULL | Story title |
| `description` | text | nullable | Story description |
| `cover_art` | text | nullable | Cover art URL |
| `genre` | text | nullable | Story genre (reserved for future use) |
| `modules` | jsonb | NOT NULL | Story module configuration |
| `created_at` | timestamp | NOT NULL, default now() | Creation timestamp |
| `updated_at` | timestamp | NOT NULL | Last update timestamp |

**Indexes:**

```typescript
uniqueIndex("stories_author_story_version_idx").on(table.authorId, table.storyId, table.version)
```

**Relations:**

```typescript
export const storyRelations = relations(stories, ({ one }) => ({
  author: one(user, {
    fields: [stories.authorId],
    references: [user.id],
  }),
}));
```

### Removed Tables

The following tables were removed in migration `0008_futuristic_changeling.sql` as part of the shift to client-side gameplay:

- `game_sessions` — replaced by `local_sessions` in SQLite
- `game_session_pages` — replaced by `local_pages` in SQLite
- `game_session_messages` — removed; messages stored in `local_pages`
- `module_runtime` — replaced by `local_module_runtime` in SQLite
- `is_vignette` column on `stories` — vignettes are no longer server-persisted

## Server Schema Relations

### Relation Graph

```
users (1) ----< (N) stories
```

The server schema is now minimal — only auth tables and stories. All gameplay relations have moved to the local SQLite schema.

## Runtime Validation with drizzle-zod

The project uses `drizzle-zod` to generate Zod validation schemas from Drizzle table definitions for runtime validation.

### Generated Insert Schemas

```typescript
import { insertStorySchema } from '~/server/db/schema/app';
```

### Usage Examples

#### Basic Validation

```typescript
import { insertStorySchema } from '#server/db/schema/app';
import { db } from '#server/db';
import { stories } from '#server/db/schema';

const storyData = {
  storyId: 'uuid-123',
  authorId: 'user-uuid',
  version: 1,
  title: 'My Story',
  description: 'A description',
  modules: { modules: [] },
};

const validated = insertStorySchema.parse(storyData);
await db.insert(stories).values(validated).returning();
```

#### Custom Validation Extension

```typescript
import { insertStorySchema } from '#server/db/schema/app';
import { z } from 'zod';

const customStorySchema = insertStorySchema.extend({
  title: z.string().min(1).max(200),
  version: z.number().int().positive(),
});

const validated = customStorySchema.parse(data);
```

#### API Route Validation

```typescript
// server/api/stories.post.ts
import { insertStorySchema } from '#server/db/schema/app';
import { db } from '#server/db';
import { stories } from '#server/db/schema';

export default defineEventHandler(async (event) => {
  const body = await readBody(event);
  const validated = insertStorySchema.parse(body);
  const story = await db.insert(stories).values(validated).returning();
  return story[0];
});
```

### Validation Schema Types

| Schema Type | Description |
|-------------|-------------|
| `insertStorySchema` | Validates data for INSERT operations |
| `selectStorySchema` | Validates data from SELECT operations |
| `updateStorySchema` | Validates data for UPDATE operations |

---

## Local Schema (SQLite via PowerSync)

Client-side gameplay state is stored in local SQLite via PowerSync. The schema is defined in `shared/db/localSchema.ts` and consumed through `app/composables/useLocalDb.ts`.

**Location:** `shared/db/localSchema.ts`

### local_sessions

Stores vignette/gameplay sessions. Replaces the server-side `game_sessions` table.

**Schema Definition:**

```typescript
export const localSessions = sqliteTable('local_sessions', {
  id: text('id').primaryKey().notNull(),
  storyId: text('story_id').notNull(),
  title: text('title').notNull(),
  description: text('description'),
  createdAt: text('created_at').notNull(),
  updatedAt: text('updated_at').notNull(),
});
```

**Columns:**

| Column | Type | Constraints | Description |
|--------|------|-------------|-------------|
| `id` | text | PRIMARY KEY, NOT NULL | UUID session identifier |
| `story_id` | text | NOT NULL | Associated story ID (nullable in vignettes) |
| `title` | text | NOT NULL | Session/vignette title |
| `description` | text | nullable | Session description |
| `created_at` | text | NOT NULL | ISO timestamp |
| `updated_at` | text | NOT NULL | ISO timestamp |

### local_pages

Stores conversation pages within a session. Replaces the server-side `game_session_pages` and `game_session_messages` tables.

**Schema Definition:**

```typescript
export const localPages = sqliteTable('local_pages', {
  id: text('id').primaryKey().notNull(),
  sessionId: text('session_id').notNull(),
  system: text('system'),
  prompt: text('prompt'),
  response: text('response'),
  createdAt: text('created_at').notNull(),
});
```

**Columns:**

| Column | Type | Constraints | Description |
|--------|------|-------------|-------------|
| `id` | text | PRIMARY KEY, NOT NULL | UUID page identifier |
| `session_id` | text | NOT NULL | Parent session ID |
| `system` | text | nullable | System prompt for this page |
| `prompt` | text | nullable | User's input/prompt |
| `response` | text | nullable | AI-generated response |
| `created_at` | text | NOT NULL | ISO timestamp |

### local_module_runtime

Stores module runtime state per session. Replaces the server-side `module_runtime` table.

**Schema Definition:**

```typescript
export const localModuleRuntime = sqliteTable('local_module_runtime', {
  id: text('id').primaryKey().notNull(),
  sessionId: text('session_id').notNull(),
  moduleId: text('module_id').notNull(),
  data: text('data').notNull(),
});
```

**Columns:**

| Column | Type | Constraints | Description |
|--------|------|-------------|-------------|
| `id` | text | PRIMARY KEY, NOT NULL | UUID identifier |
| `session_id` | text | NOT NULL | Parent session ID |
| `module_id` | text | NOT NULL | Module identifier |
| `data` | text | NOT NULL | Serialized module runtime state |

### local_profiles

Stores user-defined player profiles. Profile data is local-only — never synced to the server. The active profile's fields are injected into gameplay LLM prompts.

**Schema Definition:**

```typescript
export const localProfiles = sqliteTable('local_profiles', {
  id: text('id').primaryKey().notNull(),
  name: text('name').notNull(),
  fields: text('fields').notNull(),
  active: integer('active', { mode: 'boolean' }).notNull().default(false),
  createdAt: text('created_at').notNull(),
  updatedAt: text('updated_at').notNull(),
});
```

**Columns:**

| Column | Type | Constraints | Description |
|--------|------|-------------|-------------|
| `id` | text | PRIMARY KEY, NOT NULL | UUID profile identifier |
| `name` | text | NOT NULL | Profile display name |
| `fields` | text | NOT NULL | JSON-serialized `Record<string, string>` of key-value fields |
| `active` | integer (boolean) | NOT NULL, default false | Whether this is the currently active profile |
| `created_at` | text | NOT NULL | ISO timestamp |
| `updated_at` | text | NOT NULL | ISO timestamp |

**Business rules:**
- Max 5 profiles per user
- Only one profile may be active at a time
- Default fields prepopulated: `name`, `appearance`, `interests`, `favorite color`
- Managed via `app/composables/useProfiles.ts`

### Local Schema Barrel Export

```typescript
// shared/db/localSchema.ts
export const drizzleSchema = {
  localSessions,
  localPages,
  localModuleRuntime,
  localProfiles,
};
```

### Querying Local Database

```typescript
import { useLocalDb } from '~/composables/useLocalDb';
import { localSessions, localPages } from '#shared/db/localSchema';
import { eq } from 'drizzle-orm';

const db = useLocalDb();

// Query sessions
const sessions = await db.select().from(localSessions);

// Query pages for a session
const pages = await db.select().from(localPages).where(eq(localPages.sessionId, sessionId));

// Insert a new page
await db.insert(localPages).values({
  id: crypto.randomUUID(),
  sessionId: sessionId,
  system: 'system prompt',
  prompt: 'user input',
  response: 'AI response',
  createdAt: new Date().toISOString(),
});
```

---

## Migration Commands

```json
{
  "scripts": {
    "db:generate": "drizzle-kit generate",
    "db:migrate": "drizzle-kit migrate",
    "db:push": "drizzle-kit push",
    "db:studio": "drizzle-kit studio"
  }
}
```

### Usage (Server PostgreSQL only)

```bash
# Generate migration files from schema changes
bun run db:generate

# Apply pending migrations
bun run db:migrate

# Push schema directly to database (development only)
bun run db:push

# Open Drizzle Studio for database inspection
bun run db:studio
```

### Local Schema Changes

To modify the local SQLite schema:

1. Update `shared/db/localSchema.ts`
2. PowerSync handles schema migration automatically on next client initialization (local-only mode)

## Related Documentation

- [Code Conventions](./code-conventions.md) - Query patterns and type safety
- [Project Structure](./project-structure.md) - Schema file organization
- [API Routes](./api-routes.md) - Database usage in API endpoints
- [Frontend Architecture](./frontend-architecture.md) - How the frontend uses local SQLite
