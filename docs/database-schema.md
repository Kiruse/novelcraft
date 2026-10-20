# Data Storage

This document describes the filesystem-based persistence layer used for all gameplay state in NovelCraft.

## Overview

NovelCraft is a fully offline, single-user desktop app. All data lives as **JSON files on disk**, managed by engine types (`SessionV2`, `NovelCraftConfig`). There is no database, no SQLite, no server.

- **Persistence layer** — JSON files under the platform data/config directories (each root gains a `NovelCraft` subdirectory), read/written by engine types via `util::serialize`/`util::deserialize`
- **Path resolution** — centralized in `engine/src/paths.rs` (`data_dir()`, `config_dir()`, `sessions_dir()`)
- **Version-gated deserialization** — the session file carries a `version` field validated by a custom deserializer; anything unexpected is **rejected, not misread** (legacy v1 multi-file session directories are ignored)
- **One file, one source of truth** — each save rewrites the whole session file; restoring state means reading a single file and rebuilding the gamestate from `snapshot` + replaying `turns`. The flat-file layout is also the stepping stone for a future binary format (swap the serde impl, keep the struct)

### Technology

| Layer | Technology | Access Pattern |
|-------|-----------|----------------|
| Data | JSON files in `{dataDir}/NovelCraft/` | Engine types (`engine/src/game/`) |
| Config | JSON file in `{configDir}/NovelCraft/` | `NovelCraftConfig` (`engine/src/config.rs`) |

---

## File Layout

Those files are ALL there is — no page batches, no checkpoint files, no separate profiles store.

```
{dataDir}/NovelCraft/sessions/{sessionUUID}.json
{configDir}/NovelCraft/config.json
```

### sessions/{id}.json — The Entire Session (`SessionV2`, version 2)

```json
{
  "version": 2,
  "id": "session-uuid",
  "title": "Session Title",
  "exposition": "Brief story exposition serving as the starting point",
  "created_at": "2026-01-01T00:00:00+00:00",
  "updated_at": "2026-01-01T00:00:00+00:00",
  "modules": {
    "story": { "Story": {} },
    "npcs": { "Npcs": {} },
    "player": { "Player": {} }
  },
  "profile": "profile-uuid-or-absent",
  "total_turns": 12,
  "snapshot": { "story": { "..." : "..." } },
  "turns": [ { "prompt": "What do I see in the tavern?", "responses": [ ... ], "edits": [ [ ["npcs", "Mira", "mood"], "curious" ] ] } ]
}
```

- `created_at` / `updated_at` are RFC3339 strings (custom serde in `engine/src/util/mod.rs`)
- `modules` maps module IDs to their `GameplayModule` enum variant; module IDs **must** equal the toolset namespace (snake_case struct name with the `_module` suffix stripped) — tool-call replay resolves recorded calls by this key
- `total_turns` counts every turn ever pushed, including turns since compressed away into the rolling summary
- File version must be `2`; other versions are rejected (`SessionV2::deserialize_version`), and `NovelCraftEngine::list_sessions` skips such files with a warning (legacy session *directories* are ignored)
- The `gamestate` is NOT persisted — it is rebuilt on load

#### `snapshot` — Gamestate Anchor

The gamestate **as it was just before the first retained turn**. Re-anchored whenever turns are compressed away or forked.

```json
{
  "story": {
    "timeline": "- [x] intro\n- [ ] the city",
    "summary": "Rolling story summary written by the engine ...",
    "chapters": [
      { "title": "Chapter 1", "notes": "- met a merchant\n- bought a horse\n" }
    ]
  },
  "story": { "timeline": "- [x] arrive\n- [ ] find the mayor\n" },
  "npcs": {},
  "player": {}
}
```

- Each entry is one gameplay module's JSON object; missing modules are seeded as `{}` on load (`GameState::new`)
- `turn.edits` — manual sidebar overrides made on the last page (keypath with module ID prefix → string value; empty string = delete), re-applied after that turn's tool calls during replay
- `story.summary` is written ONLY by the engine (`StoryModule::write_summary` during compression). Agents never touch it; they maintain `chapters` via the `begin_chapter` (titles & closes the active chapter, opens a fresh untitled one) and `note_events` (appends bullets to the active chapter's notes, duplicates ignored) tools. Chapters bootstrap lazily — there is always an active, untitled last chapter. The story premise is NOT gamestate — it lives on the session (`exposition`) and is injected into the system prompt
- Loading = `GameState::new(snapshot, module_ids, profile)` + `GameState::replay(turns)` — every recorded tool call is re-applied on top of the snapshot

#### `turns` — Retained Short-Term Memory

The last `2 × summary_threshold` turns of the conversation (default threshold 50 → up to 100 retained turns).

```json
[
  {
    "system": "optional DM steering instruction (rewrites)",
    "prompt": "What do I see in the tavern?",
    "responses": [
      {
        "content": "The common room falls quiet as you enter ...",
        "tool_calls": [
          { "id": "tc1", "name": "story::note_events", "arguments": "{\"notes\":\"- ...\"}" }
        ],
        "tool_responses": [["tc1", "Chapter notes updated."]]
      }
    ]
  }
]
```

- `system`, `prompt`, `content` are omitted when `None` (`skip_serializing_if`); `responses`/`tool_calls`/`tool_responses` are `#[serde(default)]` so older files without them deserialize
- One `TurnV2` per user interaction; `AgentResponse` folds an assistant message with its subsequent tool calls & tool responses (`TurnV2::assimilate`)

### config.json — Engine Config (`NovelCraftConfig`)

```json
{
  "max_agent_steps": 10,
  "system_prompt": "You are a Dungeon Master ...",
  "models": {
    "dungeon_master": { "type": "open_ai", "base_url": "http://localhost:8888/v1", "api_key": "", "model": "..." },
    "suggestions": { "type": "open_ai", "base_url": "http://localhost:8888/v1", "api_key": "", "model": "..." },
    "summarizer": { "type": "open_ai", "base_url": "http://localhost:8888/v1", "api_key": "", "model": "..." }
  },
  "summary_threshold": 50,
  "profiles": [
    { "id": "profile-uuid", "name": "Kira", "fields": { "Class": "Rogue" }, "created_at": "...", "updated_at": "..." }
  ],
  "active_profile": "profile-uuid-or-null"
}
```

- `models` is an `ExhaustiveMap<ModelPurpose, ModelConfig>` — **all three keys are required**; a missing `summarizer` key makes the whole file fail to deserialize (existing users migrate manually). `ModelConfig` is internally tagged (`"type": "open_ai"`)
- `summary_threshold` is `#[serde(default)]` (`DEFAULT_SUMMARY_THRESHOLD` = 50), so configs predating the memory rewrite still deserialize
- Profiles live **inside** config.json — there is no separate profiles store

---

## Lifecycle Notes

- **Session creation** → `NovelCraftEngine::create_session(title, exposition)`: fresh `SessionV2` with the default module set, then a full save
- **Every prompt/rewrite commit** → `SessionV2::update_turn`/`push_turn` rewrite the single session file
- **Compression** → when the retained history exceeds `2 × summary_threshold` turns, the oldest `threshold` turns are rendered to a transcript, merged into the rolling summary by a ONE-SHOT Summarizer LLM call (nothing is mutated if that call fails), drained, and the snapshot is re-anchored to the new first retained turn
- **Fork** → `SessionV2::fork(turn_index, profiles)` truncates the retained history to `turn_index + 1` and rebuilds the gamestate from snapshot + replay. Turns outside the retained window cannot be forked from

## Related Documentation

- [Project Structure](./project-structure.md) - File organization and directory layout
- [Engine API](./api-routes.md) - Engine API reference
- [GUI Architecture](./gui-architecture.md) - Dioxus screens, components, and styling conventions
