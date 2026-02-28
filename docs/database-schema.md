# Database Schema

This document describes the complete database schema, including all tables, columns, indexes, relations, and runtime validation using drizzle-zod.

## Overview

The database is organized across multiple schema files in `server/db/schema/`:

- **auth.ts** - Better-Auth user tables (user, session, account, etc.)
- **app.ts** - Application-specific tables (stories, game_sessions, etc.)
- **placeholder.ts** - Placeholder tables for development
- **index.ts** - Central export point for all schema definitions

### Technology Stack

- **ORM**: Drizzle ORM
- **Validation**: drizzle-zod (Zod schemas generated from Drizzle definitions)
- **Migration**: Drizzle Kit
- **Database**: PostgreSQL

## Schema Organization

### Import Structure

```typescript
// server/db/schema/index.ts
export * from './auth';
export * from './app';
export * from './placeholder';
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
export const stories = pgTable('stories', {
  id: serial('id').primaryKey(),
  storyId: text('story_id').notNull(),
  authorId: text('author_id').notNull().references(() => users.id, {
    onDelete: 'restrict',
  }),
  version: integer('version').notNull(),
  title: text('title').notNull(),
  description: text('description'),
  coverArt: text('cover_art'),
  genre: text('genre'),
  modules: jsonb('modules').notNull().$type<StoryModules>(),
  createdAt: timestamp('created_at').notNull().defaultNow(),
  updatedAt: timestamp('updated_at').notNull(),
});
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
export const storiesAuthorStoryVersionIdx = index('stories_author_story_version_idx')
  .on(stories.authorId, stories.storyId, stories.version);
```

**Relations:**

```typescript
export const storiesRelations = relations(stories, ({ one, many }) => ({
  author: one(users, {
    fields: [stories.authorId],
    references: [users.id],
  }),
  gameSessions: many(gameSessions),
}));
```

### game_sessions

Stores player game sessions.

**Location:** `server/db/schema/app.ts`

**Schema Definition:**

```typescript
export const gameSessions = pgTable('game_sessions', {
  id: serial('id').primaryKey(),
  playerId: text('player_id').notNull().references(() => users.id, {
    onDelete: 'cascade',
  }),
  storyId: integer('story_id').notNull().references(() => stories.id, {
    onDelete: 'cascade',
  }),
  data: jsonb('data').notNull().$type<SessionData>(),
  createdAt: timestamp('created_at').notNull().defaultNow(),
  updatedAt: timestamp('updated_at').notNull(),
});
```

**Columns:**

| Column | Type | Constraints | Description |
|--------|------|-------------|-------------|
| `id` | serial | PRIMARY KEY | Auto-incrementing primary key |
| `player_id` | text | NOT NULL, FK → users.id | Player reference (DELETE CASCADE) |
| `story_id` | integer | NOT NULL, FK → stories.id | Story reference (DELETE CASCADE) |
| `data` | jsonb | NOT NULL | Session game state |
| `created_at` | timestamp | NOT NULL, default now() | Creation timestamp |
| `updated_at` | timestamp | NOT NULL | Last update timestamp |

**Indexes:**

```typescript
export const gameSessionsPlayerUpdatedIdx = index('game_sessions_player_updated_idx')
  .on(gameSessions.playerId, gameSessions.updatedAt);
```

**Relations:**

```typescript
export const gameSessionsRelations = relations(gameSessions, ({ one, many }) => ({
  player: one(users, {
    fields: [gameSessions.playerId],
    references: [users.id],
  }),
  story: one(stories, {
    fields: [gameSessions.storyId],
    references: [stories.id],
  }),
  moduleRuntime: many(moduleRuntime),
  messages: many(gameSessionMessages),
}));
```

### module_runtime

Stores runtime state of modules.

**Location:** `server/db/schema/app.ts`

**Schema Definition:**

```typescript
export const moduleRuntime = pgTable('module_runtime', {
  id: serial('id').primaryKey(),
  gameSessionId: integer('game_session_id').notNull().references(() => gameSessions.id, {
    onDelete: 'cascade',
  }),
  moduleId: text('module_id').notNull(),
  data: jsonb('data').notNull().$type<ModuleData>(),
});
```

**Columns:**

| Column | Type | Constraints | Description |
|--------|------|-------------|-------------|
| `id` | serial | PRIMARY KEY | Auto-incrementing primary key |
| `game_session_id` | integer | NOT NULL, FK → game_sessions.id | Session reference (DELETE CASCADE) |
| `module_id` | text | NOT NULL | Module identifier (not a FK) |
| `data` | jsonb | NOT NULL | Module runtime state |

**Indexes:**

```typescript
export const moduleRuntimeSessionIdx = index('module_runtime_session_idx')
  .on(moduleRuntime.gameSessionId);
```

**Relations:**

```typescript
export const moduleRuntimeRelations = relations(moduleRuntime, ({ one }) => ({
  gameSession: one(gameSessions, {
    fields: [moduleRuntime.gameSessionId],
    references: [gameSessions.id],
  }),
}));
```

### game_session_messages

Stores conversation history for game sessions.

**Location:** `server/db/schema/app.ts`

**Schema Definition:**

```typescript
export const messageRoleEnum = pgEnum('message_role', ['system', 'agent', 'user', 'toolcall'] as const);

export const gameSessionMessages = pgTable('game_session_messages', {
  id: serial('id').primaryKey(),
  gameSessionId: integer('game_session_id').notNull().references(() => gameSessions.id, {
    onDelete: 'cascade',
  }),
  role: messageRoleEnum('role').notNull(),
  contents: text('contents').notNull(),
  toolcallData: jsonb('toolcall_data').$type<ToolcallData>(),
  createdAt: timestamp('created_at').notNull().defaultNow(),
});
```

**Columns:**

| Column | Type | Constraints | Description |
|--------|------|-------------|-------------|
| `id` | serial | PRIMARY KEY | Auto-incrementing primary key |
| `game_session_id` | integer | NOT NULL, FK → game_sessions.id | Session reference (DELETE CASCADE) |
| `role` | enum | NOT NULL | Message role (system/agent/user/toolcall) |
| `contents` | text | NOT NULL | Message content |
| `toolcall_data` | jsonb | nullable | Tool call data |
| `created_at` | timestamp | NOT NULL, default now() | Creation timestamp |

**Indexes:**

```typescript
export const gameSessionMessagesSessionCreatedIdx = index('game_session_messages_session_created_idx')
  .on(gameSessionMessages.gameSessionId, gameSessionMessages.createdAt);
```

**Relations:**

```typescript
export const gameSessionMessagesRelations = relations(gameSessionMessages, ({ one }) => ({
  gameSession: one(gameSessions, {
    fields: [gameSessionMessages.gameSessionId],
    references: [gameSessions.id],
  }),
}));
```

## Enums

### message_role

Enum defining message roles for game session messages.

**Values:**
- `'system'` - System-level messages
- `'agent'` - AI agent responses
- `'user'` - User messages
- `'toolcall'` - Tool/function call records

**Definition:**

```typescript
export const messageRoleEnum = pgEnum('message_role', ['system', 'agent', 'user', 'toolcall'] as const);
```

## Schema Relations

All foreign key relations are defined using Drizzle ORM's `relations` API for type-safe queries.

### Example Query with Relations

```typescript
import { db } from '~/server/db';
import { stories } from '~/server/db/schema';
import { eq } from 'drizzle-orm';

// Query story with author and game sessions
const storyWithAuthor = await db.query.stories.findFirst({
  where: eq(stories.id, 1),
  with: {
    author: true,
    gameSessions: {
      with: {
        player: true,
        messages: true,
      },
    },
  },
});
```

### Relation Graph

```
users (1) ----< (1) stories
  |                     |
  |                     | (1)
  |                     +----< (N) game_sessions ----< (N) game_session_messages
  |                     |
  |                     | (N)
  |                     +----< (N) module_runtime
  |
  +----< (N) game_sessions
```

## Runtime Validation with drizzle-zod

The project uses `drizzle-zod` to generate Zod validation schemas from Drizzle table definitions for runtime validation.

### Generated Insert Schemas

drizzle-zod automatically generates Zod schemas for insert operations:

```typescript
import {
  insertStorySchema,
  insertGameSessionSchema,
  insertModuleRuntimeSchema,
  insertGameSessionMessageSchema,
} from '~/server/db/schema/app';
import { z } from 'zod';
```

### Usage Examples

#### Basic Validation

```typescript
import { insertStorySchema } from '~/server/db/schema/app';
import { db } from '~/server/db';
import { stories } from '~/server/db/schema';

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
await db.insert(stories).values(validated).returning();
```

#### Custom Validation Extension

Extend generated schemas with additional validation rules:

```typescript
import { insertStorySchema } from '~/server/db/schema/app';
import { z } from 'zod';

const customStorySchema = insertStorySchema.extend({
  title: z.string().min(1).max(200),
  version: z.number().int().positive(),
  genre: z.enum(['fantasy', 'sci-fi', 'mystery']).optional(),
});

const validated = customStorySchema.parse(data);
```

#### API Route Validation

```typescript
// server/api/stories.post.ts
import { insertStorySchema } from '~/server/db/schema/app';
import { db } from '~/server/db';
import { stories } from '~/server/db/schema';

export default defineEventHandler(async (event) => {
  const body = await readBody(event);
  const validated = insertStorySchema.parse(body);
  const story = await db.insert(stories).values(validated).returning();
  return story[0];
});
```

#### Select Schema Validation

drizzle-zod also generates select schemas for output validation:

```typescript
import { selectStorySchema } from '~/server/db/schema/app';

const story = await db.select().from(stories).limit(1);
const validated = selectStorySchema.parse(story[0]);
```

### Validation Schema Types

| Schema Type | Description |
|-------------|-------------|
| `insertStorySchema` | Validates data for INSERT operations |
| `selectStorySchema` | Validates data from SELECT operations |
| `updateStorySchema` | Validates data for UPDATE operations |

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

### Usage

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

## Related Documentation

- [Code Conventions](./code-conventions.md) - Query patterns and type safety
- [Project Structure](./project-structure.md) - Schema file organization
- [API Routes](./api-routes.md) - Database usage in API endpoints
