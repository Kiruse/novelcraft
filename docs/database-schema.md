# Data Storage

This document describes the filesystem-based persistence layer used for all gameplay state in NovelCraft.

## Overview

NovelCraft is a fully offline, single-user desktop app. All data lives as **JSON files on disk**, managed by the Rust engine library. There is no database, no SQLite, no server.

- **Persistence layer** — JSON files in the platform data directory (resolved via `dirs::data_dir()`), read/written by engine command functions
- **Path resolution** — centralized in `engine/src/commands/paths.rs` via `data_dir()` and `config_dir()`
- **Version-gated deserialization** — Every file format includes a `version` field for forward-compatible schema evolution
- **No transactions** — Each engine function call performs a single atomic file operation. Consumers drive sequential operations when multiple steps are needed.

### Technology

| Layer | Technology | Access Pattern |
|-------|-----------|----------------|
| Storage | JSON files in `{dataDir}/` | Rust engine functions (`engine/src/commands/`) |
| Config | JSON files in `{configDir}/` | Rust engine functions |

---

## File Layout

All data is stored under the platform data directory resolved by `engine/src/commands/paths.rs::data_dir()`.

### Stories

```
{dataDir}/stories/{id}.json
```

Each file contains a single `StoryEntry`:

```json
{
  "version": 1,
  "id": "uuid",
  "title": "Story Title",
  "description": "Optional description",
  "config": {},
  "created_at": "2025-01-01T00:00:00Z",
  "updated_at": "2025-01-01T00:00:00Z"
}
```

### Sessions (Vignettes)

```
{dataDir}/sessions/{sessionUUID}/meta.json
{dataDir}/sessions/{sessionUUID}/pages.{batch:03}.json
{dataDir}/sessions/{sessionUUID}/state.head.json
{dataDir}/sessions/{sessionUUID}/state.{batch:03}.json
```

#### meta.json — Session Metadata

```json
{
  "version": 1,
  "id": "session-uuid",
  "story_id": null,
  "title": "Session Title",
  "description": "Opening text",
  "disposition": "Opening disposition",
  "created_at": "2025-01-01T00:00:00Z",
  "updated_at": "2025-01-01T00:00:00Z"
}
```

#### pages.{batch}.json — Pages Batch

Pages are stored in batches of 100. The batch index for page N is `floor(N / 100)`. Files are zero-padded to 3 digits: `pages.000.json`, `pages.001.json`, etc.

```json
{
  "version": 1,
  "batch": 0,
  "pages": [
    {
      "id": "page-uuid",
      "system": "System prompt for this page",
      "prompt": "User input",
      "response": "AI response",
      "tool_calls": "[{"tool":"npc::addNPC","params":{"name":"Alice"}}]",
      "created_at": "2025-01-01T00:00:00Z"
    }
  ]
}
```

#### state.head.json — Head Snapshot

The current gameplay state, updated on every page completion:

```json
{
  "version": 1,
  "page_index": 5,
  "data": { "npc": { "version": 1, "npcs": [] }, "plan": { "version": 1 } }
}
```

#### state.{batch}.json — Checkpoint Snapshots

Checkpoint snapshots created every 100 pages. The batch index matches the page batch: `state.000.json`, `state.001.json`, etc.

### Profiles

```
{dataDir}/profiles.json
```

A single file containing all profiles and the active profile ID. Held in memory via `OnceCell<Mutex<ProfilesFile>>` in `engine/src/commands/profile.rs`, initialized by `init_profiles()`.

```json
{
  "version": 1,
  "profiles": [
    {
      "id": "profile-uuid",
      "name": "Default",
      "fields": { "name": "", "appearance": "", "interests": "", "favorite color": "" },
      "created_at": "2025-01-01T00:00:00Z",
      "updated_at": "2025-01-01T00:00:00Z"
    }
  ],
  "active_id": "profile-uuid"
}
```

**Business rules:**
- Max 5 profiles per user
- Active profile tracked at the file level via `active_id` (not per-profile)
- `profile_delete` clears `active_id` if the deleted profile was active
- Default fields: `name`, `appearance`, `interests`, `favorite color`

### Lore Entries

```
{dataDir}/lore/{id}.json
```

Each file contains a single `LoreEntry`:

```json
{
  "version": 1,
  "id": "lore-uuid",
  "story_id": "story-uuid",
  "title": "Lore Title",
  "content": "Lore content",
  "tags": ["tag1", "tag2"],
  "created_at": "2025-01-01T00:00:00Z",
  "updated_at": "2025-01-01T00:00:00Z"
}
```

### Model Configuration

```
{dataDir}/models.json
```

A single file containing the LLM model configuration. Held in memory in `AppState.models: Mutex<Models>`, initialized via `Models::load()` in `AppState::init()`. The `Models` struct is defined in `engine/src/commands/llm.rs`.

```json
{
  "storyteller": {
    "model_id": "zai-org/glm-4.6v-flash",
    "base_url": "http://localhost:1234/v1",
    "api_key": null
  },
  "suggestions": {
    "model_id": "zai-org/glm-4.6v-flash",
    "base_url": "http://localhost:1234/v1",
    "api_key": null
  }
}
```

- Default models point to `http://localhost:1234/v1` (local LLM server)
- `Models::patch()` fills empty `base_url` fields with the default host

---

## Snapshot Lifecycle

State snapshots enable undo/fork by capturing module state at each page:

- **Session creation** → head snapshot with empty state `{}`
- **After each page's tool calls** → head snapshot (`state.head.json`) updated in place
- **Every 100 pages** → copy current head as checkpoint (`state.{batch}.json`) before updating
- **Fork** → `GameEngine::fork(page_index)` truncates page batch files after the batch containing `page_index`, then reloads the session to rebuild state
- **State is immutable** — the canonical state is the snapshot data
- **Initial module state** is always empty (`{}`); first tool call populates what's needed via `init()` fallback

---

## Version-Gated Deserialization

Every JSON file includes a `version` field. The engine uses `read_versioned_json()` to:

1. Deserialize the JSON into a generic `Value`
2. Extract the `version` field
3. Match on the version number
4. Dispatch to the correct typed deserializer (currently only v1)

This allows future format changes to add new match arms without breaking existing files.

---

## Data Access from Engine

All data access goes through engine command functions — never direct file I/O from the GUI layer.

```rust
use novelcraft_engine::AppState;
use novelcraft_engine::commands::{session, profile, story, lore};

// Sessions
let sessions = session::session_list(&app).await?;
let result = session::session_load(&app, id).await?;
session::session_create(&app, id, story_id, title, description).await?;
session::session_delete(&app, id).await?;
session::session_save_meta(&app, id, meta).await?;
session::session_push_page(&app, id, page_entry).await?;
session::session_update_page(&app, id, 0, page_entry).await?;
session::session_truncate_pages(&app, id, 5).await?;

// Snapshots
let head = session::session_get_head_snapshot(&app, id).await?;
session::session_save_head_snapshot(&app, id, snapshot).await?;
session::session_delete_head_snapshot(&app, id).await?;
let snap = session::session_find_snapshot_before(&app, id, 5).await?;
session::session_save_checkpoint(&app, id, 0, snap).await?;
session::session_delete_checkpoints_from(&app, id, 5).await?;

// Profiles
let result = profile::profile_list(&app).await?;
profile::profile_create(&app, new_profile).await?;
profile::profile_update(&app, profile_id, updated_profile).await?;
profile::profile_delete(&app, profile_id).await?;
profile::profile_set_active(&app, profile_id).await?;

// Stories
let story = story::story_get(&app, id).await?;
story::story_save(&app, story_entry).await?;

// Lore
let results = lore::lore_query(&app, id, "search term").await?;
```

---

## Advantages

- **No connection pool issues**: No database connection management
- **Easy inspection**: JSON files can be read with any text editor or tool
- **Simple backup**: Copy the entire `{dataDir}/` directory
- **Future "mod" support**: Loose file structure enables loading external data files
- **No migration system needed**: Version-gated deserialization handles schema evolution inline

---

## Related Documentation

- [Code Conventions](./code-conventions.md) - Import patterns and async functions
- [Project Structure](./project-structure.md) - File organization for persistence commands
- [Engine API](./api-routes.md) - Engine command function reference
- [GUI Architecture](./gui-architecture.md) - gpui GUI components and screens