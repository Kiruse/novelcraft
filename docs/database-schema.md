# Data Storage

This document describes the filesystem-based persistence layer used for all gameplay state in Novelcraft.

## Overview

NovelCraft is a fully offline desktop app. All data lives as **JSON files on disk**, managed by the Rust backend through Tauri commands. There is no database, no SQLite, no server.

- **Persistence layer** — JSON files in the Tauri app data directory, read/written by Rust backend commands
- **Frontend access** — All data access goes through tauri-specta generated bindings (`commands.xxx()` from `gui/src/bindings.ts`), wrapped with `unwrap()` from `gui/src/utils/index.ts`. The only raw `invoke()` remaining is `invoke('prompt', ...)` in `tauriLanguageModel.ts` for LLM streaming.
- **Version-gated deserialization** — Every file format includes a `version` field for forward-compatible schema evolution

### Technology Stack

| Layer | Technology | Access Pattern |
|-------|-----------|----------------|
| Storage | JSON files in `{appData}/` | Rust backend via Tauri commands |
| Frontend | `commands.xxx()` from `~/bindings` + `unwrap()` from `~/utils` | Composables in `gui/src/composables/` |

---

## File Layout

All data is stored under the Tauri app data directory (`{appData}/`). The layout uses loose JSON files organized by entity type.

### Stories

```
{appData}/stories/{id}.json
```

Each file contains a single `StoryEntry`:

```typescript
{
  version: 1;
  id: string;           // UUID
  title: string;
  description?: string;
  config: Record<string, unknown>;  // { [moduleType]: moduleConfig }
  created_at: string;   // ISO timestamp
  updated_at: string;   // ISO timestamp
}
```

### Sessions (Vignettes)

```
{appData}/sessions/{sessionUUID}/meta.json
{appData}/sessions/{sessionUUID}/pages.{batch:03}.json
{appData}/sessions/{sessionUUID}/state.head.json
{appData}/sessions/{sessionUUID}/state.{batch:03}.json
```

#### meta.json — Session Metadata

```typescript
{
  version: 1;
  id: string;               // UUID session identifier
  story_id?: string | null; // Associated story ID (absent for impromptu/freeform sessions)
  title: string;
  description?: string;     // Opening text for the session
  disposition?: string;     // Opening text for the session
  created_at: string;       // ISO timestamp
  updated_at: string;       // ISO timestamp
}
```

#### pages.{batch}.json — Pages Batch

Pages are stored in batches of 100. The batch index for page N is `floor(N / 100)`. Files are zero-padded to 3 digits: `pages.000.json`, `pages.001.json`, etc.

```typescript
// PagesBatch
{
  version: 1;
  batch: number;        // Batch index (0, 1, 2, ...)
  pages: PageEntry[];
}

// PageEntry
{
  id: string;           // UUID page identifier
  system?: string;      // System prompt for this page
  prompt?: string;      // User's input/prompt
  response?: string;    // AI-generated response
  tool_calls?: string;  // JSON array of ToolCallRecord objects: { tool: string, params: Record<string, unknown> }
  created_at: string;   // ISO timestamp
}
```

#### state.head.json — Head Snapshot

The current gameplay state, updated on every page completion:

```typescript
{
  version: 1;
  page_index: number;   // Page index this snapshot corresponds to
  data: Record<string, unknown>;  // { [moduleType]: moduleState }
}
```

#### state.{batch}.json — Checkpoint Snapshots

Checkpoint snapshots created every 100 pages. The batch index matches the page batch: `state.000.json`, `state.001.json`, etc.

```typescript
// Snapshot (generated binding type)
{
  version: 1;
  page_index: number;
  data: Record<string, unknown>;  // { [moduleType]: moduleState }
}
```

### Profiles

```
{appData}/profiles.json
```

A single file containing all profiles and the active profile ID. Held in memory via `OnceCell<Mutex<ProfilesFile>>` in `engine/src/src/commands/profile.rs`, initialized by `init_profiles()` called in `lib.rs` setup.

```typescript
// ProfilesFile
{
  version: 1;
  profiles: Profile[];
  active_id: string | null;  // ID of the active profile, or null if none set
}

// Profile
{
  id: string;           // UUID profile identifier
  name: string;         // Display name
  fields: any;          // Key-value fields (serde_json::Value, exported as TS any)
  created_at: string;   // ISO timestamp
  updated_at: string;   // ISO timestamp
}
```

**Business rules:**
- Max 5 profiles per user
- Active profile tracked at the file level via `active_id` (not per-profile)
- `profile_delete` clears `active_id` if the deleted profile was active
- Default fields prepopulated: `name`, `appearance`, `interests`, `favorite color`
- Managed via `gui/src/composables/useProfiles.ts`

### Lore Entries

```
{appData}/lore/{id}.json
```

Each file contains a single `LoreEntry`:

```typescript
{
  version: 1;
  id: string;           // UUID entry identifier
  story_id: string;     // Parent story ID
  title: string;
  content: string;
  tags?: string[];      // Array of strings for categorization
  created_at: string;   // ISO timestamp
  updated_at: string;   // ISO timestamp
}
```

---

## Snapshot Lifecycle

State snapshots enable undo/fork by capturing module state at each page:

- **Session creation** → head snapshot with empty state `{}`
- **After each page's tool calls** → head snapshot (`state.head.json`) updated in place
- **Every 100 pages** → copy current head as checkpoint (`state.{batch}.json`) before updating
- **Fork** → delete head snapshot + checkpoints with `page_index >= fork_index`, recompute head from youngest checkpoint before `fork_index` via tool call replay through `registry.executeTool()` (with `init()` fallback for uninitialized module state), then `push()` to create a new page
- **State is immutable** — immer drafts are used for convenience during tool execution and replay (inside `executeTool()`), but the canonical state is the snapshot data
- **Initial module state** is always empty (`{}`); first tool call populates what's needed via `init()` fallback (forward-compatible with module upgrades). `executeTool()` applies this fallback when module state is undefined.

---

## Version-Gated Deserialization

Every JSON file includes a `version` field. The Rust backend uses `read_versioned_json()` to:

1. Deserialize the JSON into a generic `Value`
2. Extract the `version` field
3. Match on the version number
4. Dispatch to the correct typed deserializer (currently only v1)

This allows future format changes to add new match arms without breaking existing files.

---

## Data Access from Frontend

All data access goes through tauri-specta generated bindings — never direct file I/O from the frontend. The `unwrap()` helper converts the `typedError` discriminated union back to throw-on-error behavior.

```typescript
import { commands } from '~/bindings';
import { unwrap } from '~/utils';

// Sessions
const sessions = await unwrap(commands.sessionList());
const result = await unwrap(commands.sessionLoad(id));
await unwrap(commands.sessionCreate(id, storyId ?? null, 'New Session'));
await unwrap(commands.sessionDelete(id));
await unwrap(commands.sessionSaveMeta(id, meta));
await unwrap(commands.sessionPushPage(id, pageEntry));
await unwrap(commands.sessionUpdatePage(id, 0, pageEntry));
await unwrap(commands.sessionTruncatePages(id, 5));

// Snapshots
const head = await unwrap(commands.sessionGetHeadSnapshot(id));
await unwrap(commands.sessionSaveHeadSnapshot(id, snapshotEntry));
await unwrap(commands.sessionDeleteHeadSnapshot(id));
const snap = await unwrap(commands.sessionFindSnapshotBefore(id, 5));
await unwrap(commands.sessionSaveCheckpoint(id, 0, snapshotEntry));
await unwrap(commands.sessionDeleteCheckpointsFrom(id, 5));

// Profiles
const { profiles, active_id } = await unwrap(commands.profileList());
await unwrap(commands.profileCreate(profileEntry));
await unwrap(commands.profileUpdate(profileId, updatedEntry));
await unwrap(commands.profileDelete(profileId));
await unwrap(commands.profileSetActive(profileId));

// Stories
const story = await unwrap(commands.storyGet(id));
await unwrap(commands.storySave(storyEntry));

// Lore
const results = await unwrap(commands.loreQuery(id, 'search term'));
```

**No transactions**: Each file operation is independent. The frontend drives sequential operations. There is no `db.transaction()` — each bindings call performs a single atomic file read or write.

---

## Composables Mapping

| Composable | Tauri Commands Used |
|-----------|-------------------|
| `useVignettes` | `session_list`, `session_create`, `session_delete` |
| `useVignette` | `session_load`, `session_save_meta`, `session_push_page`, `session_update_page`, `session_truncate_pages`, `session_get_head_snapshot`, `session_save_head_snapshot`, `session_delete_head_snapshot`, `session_find_snapshot_before`, `session_save_checkpoint`, `session_delete_checkpoints_from` |
| `useProfiles` | `profile_list`, `profile_create`, `profile_update`, `profile_delete`, `profile_set_active` |
| `useStoryBuilder` | `story_get`, `story_save` |
| `loreModule` | `lore_query` |

---

## Profile Fields in Prompts

The active profile's fields are injected into story/gameplay LLM calls (vignette opening, write, steer, instruct) as a `[Player profile]` block in the context message via `buildProfileContext()` in `gui/src/composables/useGame.ts`. Profile fields are NOT injected into suggestion prompts or story metadata prompts.

---

## Advantages Over SQLite

- **No connection pool issues**: Eliminates the `db.transaction()` bug where sqlite-proxy + sqlx pool connection routing broke transaction semantics
- **Easy inspection**: JSON files can be read with any text editor or tool
- **Simple backup**: Copy the entire `{appData}/` directory
- **Future "mod" support**: Loose file structure enables loading external data files
- **No migration system needed**: Version-gated deserialization handles schema evolution inline

---

## Related Documentation

- [Code Conventions](./code-conventions.md) - Import patterns and async functions
- [Project Structure](./project-structure.md) - File organization for persistence commands
- [API Routes](./api-routes.md) - Full Tauri command documentation with parameters and return types
- [Frontend Architecture](./frontend-architecture.md) - How the frontend uses Tauri commands for data access
