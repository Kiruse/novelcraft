# Database Schema

This document describes the local SQLite database schema used for all gameplay state in Novelcraft.

## Overview

NovelCraft is a fully offline desktop app. All data lives in a **single local SQLite database** accessed via `tauri-plugin-sql`. There is no server database, no PostgreSQL, no sync.

- **Local database** (SQLite via `tauri-plugin-sql`) — gameplay state: vignettes/sessions, pages, module runtime, profiles
- **DB file**: `sqlite:novelcraft.db` (path managed by Tauri plugin)
- **Access**: Raw SQL through `select<T>()` and `execute()` helpers from `gui/src/composables/useLocalDb.ts`

### Technology Stack

| Layer | Technology | Access Pattern |
|-------|-----------|----------------|
| Client | SQLite via `tauri-plugin-sql` | Raw SQL via `select<T>()` / `execute()` |

---

## Local Schema (SQLite via tauri-plugin-sql)

Client-side gameplay state is stored in local SQLite. Tables are created via `CREATE TABLE IF NOT EXISTS` in `gui/src/composables/useLocalDb.ts`.

### local_sessions

Stores vignette/gameplay sessions.

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

Stores conversation pages within a session. Each page holds one user prompt and one AI response.

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

Stores module runtime state per session.

**Columns:**

| Column | Type | Constraints | Description |
|--------|------|-------------|-------------|
| `id` | text | PRIMARY KEY, NOT NULL | UUID identifier |
| `session_id` | text | NOT NULL | Parent session ID |
| `module_id` | text | NOT NULL | Module identifier |
| `data` | text | NOT NULL | Serialized module runtime state |

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

## Querying Local Database

All queries use raw SQL through auto-imported helpers.

```typescript
// select() and execute() are auto-imported globally (declared in gui/src/env.d.ts)

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

## Schema Changes

To modify the local SQLite schema:

1. Add/modify the `CREATE TABLE IF NOT EXISTS` SQL in `gui/src/composables/useLocalDb.ts`
2. Schema changes apply on next app launch
3. Tables are created if not exist
4. Column additions require manual `ALTER TABLE` or DB recreation

## Related Documentation

- [Code Conventions](./code-conventions.md) - Query patterns and type safety
- [Project Structure](./project-structure.md) - Schema file organization
- [API Routes](./api-routes.md) - Tauri command documentation
- [Frontend Architecture](./frontend-architecture.md) - How the frontend uses local SQLite
