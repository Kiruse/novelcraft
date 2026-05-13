# Database Schema

This document describes the local SQLite database schema used for all gameplay state in Novelcraft.

## Overview

NovelCraft is a fully offline desktop app. All data lives in a **single local SQLite database** accessed via `tauri-plugin-sql` through **Drizzle ORM**. There is no server database, no PostgreSQL, no sync.

- **Local database** (SQLite via `tauri-plugin-sql`) — gameplay state: stories, sessions, pages, state snapshots, lore entries, profiles, onboarding
- **DB file**: `sqlite:novelcraft.db` (path managed by Tauri plugin)
- **Access**: Drizzle ORM through `db` instance and table references from `gui/src/db/`

### Technology Stack

| Layer | Technology | Access Pattern |
|-------|-----------|----------------|
| Client | SQLite via `tauri-plugin-sql` | Drizzle ORM via `db` instance from `gui/src/db/` |

---

## Local Schema (SQLite via Drizzle ORM)

Client-side gameplay state is stored in local SQLite. Tables are defined in `gui/src/db/schema.ts` using Drizzle ORM's `sqliteTable()`. Schema evolution is managed by Drizzle Kit migrations (see [Migrations](#migrations) below).

### local_stories

Stores story metadata and module configuration. Each story defines a set of gameplay modules and their config.

**Columns:**

| Column | Type | Constraints | Description |
|--------|------|-------------|-------------|
| `id` | text | PRIMARY KEY, NOT NULL | UUID story identifier |
| `title` | text | NOT NULL | Story title |
| `description` | text | nullable | Story description |
| `config` | text | NOT NULL | JSON blob: `{ [moduleType]: moduleConfig }` — maps module type strings to their configuration |
| `created_at` | text | NOT NULL | ISO timestamp |
| `updated_at` | text | NOT NULL | ISO timestamp |

### local_sessions

Stores vignette/gameplay sessions. Each session is tied to a story.

**Columns:**

| Column | Type | Constraints | Description |
|--------|------|-------------|-------------|
| `id` | text | PRIMARY KEY, NOT NULL | UUID session identifier |
| `story_id` | text | NOT NULL | Associated story ID |
| `title` | text | NOT NULL | Session/vignette title |
| `description` | text | nullable | Session description |
| `created_at` | text | NOT NULL | ISO timestamp |
| `updated_at` | text | NOT NULL | ISO timestamp |

### local_pages

Stores conversation pages within a session. Each page holds one user prompt, one AI response, and optionally a record of tool calls made during that response.

**Columns:**

| Column | Type | Constraints | Description |
|--------|------|-------------|-------------|
| `id` | text | PRIMARY KEY, NOT NULL | UUID page identifier |
| `session_id` | text | NOT NULL | Parent session ID |
| `system` | text | nullable | System prompt for this page |
| `prompt` | text | nullable | User's input/prompt |
| `response` | text | nullable | AI-generated response |
| `tool_calls` | text | nullable | JSON array of `ToolCallRecord` objects: `{ tool: string, params: Record<string, unknown> }` |
| `created_at` | text | NOT NULL | ISO timestamp |

### local_state_snapshots

Stores gameplay state snapshots per session. Each snapshot captures the full module state (`{ [moduleType]: moduleState }`) at a specific page index.

**Columns:**

| Column | Type | Constraints | Description |
|--------|------|-------------|-------------|
| `id` | text | PRIMARY KEY, NOT NULL | UUID snapshot identifier |
| `session_id` | text | NOT NULL | Parent session ID |
| `page_index` | integer | NOT NULL | Page index this snapshot corresponds to |
| `data` | text | NOT NULL | JSON blob: `{ [moduleType]: moduleState }` — maps module type strings to their serialized state |
| `created_at` | text | NOT NULL | ISO timestamp |

**Snapshot lifecycle:**

- **Head snapshot**: The row with the highest `page_index` for a given `session_id`. Represents the current gameplay state.
- **Checkpoints**: Snapshots at every 100-page boundary are kept permanently. Non-checkpoint snapshots are cleaned up after each `push()` finalization. Enable state reconstruction after forks.
- **Creation**: Session creation inserts snapshot 0 with empty state `{}`. After each page's tool calls complete, the head snapshot is updated in place (or a new snapshot is inserted at checkpoint boundaries).
- **Checkpointing**: Every 100 pages, a new snapshot is inserted (rather than updating the head in place). Non-checkpoint snapshots (those not on a 100-page boundary) are cleaned up after each push.
- **Fork**: When a fork occurs at `page_index`, all snapshots with `page_index >= fork_index` are deleted. The head is recomputed from the youngest snapshot with `page_index < fork_index` by replaying tool calls from surviving pages.
- **State is immutable** — immer drafts are used for convenience during tool execution and replay, but the canonical state is the snapshot data.
- **Initial module state** is always empty (`{}`); first tool call populates what's needed. This is forward-compatible with module upgrades.

### local_lore_entries

Stores lore/knowledge entries for the lore module. Entries are queryable by the `lore::query` tool during gameplay.

**Columns:**

| Column | Type | Constraints | Description |
|--------|------|-------------|-------------|
| `id` | text | PRIMARY KEY, NOT NULL | UUID entry identifier |
| `story_id` | text | NOT NULL | Parent story ID |
| `title` | text | NOT NULL | Entry title |
| `content` | text | NOT NULL | Entry content body |
| `tags` | text | nullable | JSON array of strings for categorization |
| `created_at` | text | NOT NULL | ISO timestamp |
| `updated_at` | text | NOT NULL | ISO timestamp |

### local_profiles

Stores user-defined player profiles. Profile data is local-only. The active profile's fields are injected into gameplay LLM prompts.

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
- Managed via `gui/src/composables/useProfiles.ts`

### local_onboarding

Stores whether the first-run onboarding has been completed. Single-row table.

**Columns:**

| Column | Type | Constraints | Description |
|--------|------|-------------|-------------|
| `completed` | integer (boolean) | NOT NULL, default 0 | Whether onboarding is complete (0 = not done, 1 = done) |

**Business rules:**
- Exactly one row — created on first check if not present
- Managed via `gui/src/composables/useOnboarding.ts`

## Querying Local Database

All queries use Drizzle ORM through `db` and table references from `gui/src/db/`.

```typescript
import { db, localSessions, localPages, localStateSnapshots, localLoreEntries } from '~/db';
import { eq, desc, and, like, or } from 'drizzle-orm';

// Query stories
const stories = await db.select().from(localStories).orderBy(desc(localStories.createdAt));

// Query sessions for a story
const sessions = await db.select().from(localSessions)
  .where(eq(localSessions.storyId, storyId))
  .orderBy(desc(localSessions.createdAt));

// Query pages with tool calls
const pages = await db.select().from(localPages)
  .where(eq(localPages.sessionId, sessionId))
  .orderBy(localPages.createdAt);

// Query head snapshot for a session
const snapshot = await db.select().from(localStateSnapshots)
  .where(eq(localStateSnapshots.sessionId, sessionId))
  .orderBy(desc(localStateSnapshots.pageIndex))
  .limit(1);

// Query lore entries for a story
const lore = await db.select().from(localLoreEntries)
  .where(
    and(
      eq(localLoreEntries.storyId, storyId),
      or(
        like(localLoreEntries.title, `%${query}%`),
        like(localLoreEntries.content, `%${query}%`)
      )
    )
  );

// Insert a story
await db.insert(localStories).values({
  id, title, description, config: JSON.stringify(config),
  createdAt: new Date().toISOString(), updatedAt: new Date().toISOString(),
});

// Insert a snapshot
await db.insert(localStateSnapshots).values({
  id, sessionId, pageIndex, data: JSON.stringify(state),
  createdAt: new Date().toISOString(),
});

// Update
await db.update(localSessions)
  .set({ title: newTitle, updatedAt: new Date().toISOString() })
  .where(eq(localSessions.id, sessionId));

// Delete
await db.delete(localSessions).where(eq(localSessions.id, sessionId));
```

## Migrations

Schema evolution is managed by **Drizzle Kit** migrations. Migration SQL files live in `gui/drizzle/` and are bundled at build time via `import.meta.glob`. They are applied automatically on first DB connection by `runMigrations()` in `gui/src/db/index.ts`.

### Configuration

`gui/drizzle.config.ts` configures Drizzle Kit:

```typescript
import { defineConfig } from 'drizzle-kit';

export default defineConfig({
  schema: './src/db/schema.ts',
  out: './drizzle',
  dialect: 'sqlite',
});
```

### Migration Files

Generated migrations are stored in `gui/drizzle/`:

- `0000_legal_venom.sql` — Initial migration that creates all 7 tables
- `meta/_journal.json` — Drizzle Kit migration journal (tracks migration order and checksums)

Each migration SQL file may contain multiple statements separated by `--> statement-breakpoint` markers. The migration runner splits on this marker and executes each statement sequentially.

### Workflow

1. Modify `gui/src/db/schema.ts` (add/modify table definitions)
2. Run `just generate-migration` — generates a new numbered SQL migration in `gui/drizzle/`
3. Migration is automatically applied on next app launch

```bash
# After modifying schema.ts
just generate-migration
```

### How Migrations Run

The migration runner in `gui/src/db/index.ts` (`runMigrations()`):

1. Creates a `_migrations` tracking table (`id`, `name`, `applied_at`) if it does not exist
2. Queries already-applied migration names from `_migrations`
3. For each pending migration (not yet applied):
   - Splits the SQL on `--> statement-breakpoint`
   - Executes each statement sequentially
   - Records the migration as applied in `_migrations`
4. Migrations are bundled at build time via `import.meta.glob('../../drizzle/*.sql', { query: '?raw', eager: true, import: 'default' })` — no runtime file I/O needed

### Baseline Detection

For databases created before the migration system existed:

1. The runner checks if `_migrations` table exists AND `local_stories` table exists
2. If `local_stories` exists but no migrations have been applied → the database is a pre-migration database
3. All current migrations are marked as applied (inserted into `_migrations`) without executing their SQL
4. This prevents re-creating tables that already exist

### Adding Schema Changes

To modify the local SQLite schema:

1. Add/modify table definitions in `gui/src/db/schema.ts` (Drizzle `sqliteTable()` definitions)
2. Run `just generate-migration` to generate a new migration SQL file
3. The migration runs automatically on the next app launch
4. No manual `ALTER TABLE` or DB recreation needed — Drizzle Kit generates the appropriate SQL

## Related Documentation

- [Code Conventions](./code-conventions.md) - Query patterns and type safety
- [Project Structure](./project-structure.md) - Schema file organization
- [API Routes](./api-routes.md) - Tauri command documentation
- [Frontend Architecture](./frontend-architecture.md) - How the frontend uses local SQLite
