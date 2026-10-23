# Engine API

This document describes the engine API of NovelCraft. Since this is an offline desktop app with no server, all functionality is provided as Rust methods on `NovelCraftEngine` (`engine/src/game/engine.rs`), called directly by the GUI crate through the bridge in `gui/src/engine.rs`.

## Overview

The engine crate (`novelcraft-engine`) owns the config, the active session, and the cached agent loop. There is no `AppState`, no IPC, no event bus, no code generation. All LLM transport (HTTP streaming, SSE parsing) lives inside the `kiruklaw-agent-loop` dependency.

### Capabilities

- **Session management** — create, list (metadata-only), load, fork memory-managed sessions (`SessionV2`)
- **Dungeon Master agent loop** — `prompt`/`rewrite` run a kiruklaw `AgentLoop` over the session's gameplay-module toolsets, streaming `AgentMessageChunk`s
- **Long-term memory** — rolling story summary compression (one-shot Summarizer call) + `StoryState` read model
- **Config & profiles** — `NovelCraftConfig` incl. all profiles, persisted at `{configDir}/NovelCraft/config.json`
- **Filesystem persistence** — JSON files only (see [Data Storage](./database-schema.md))

### Calling from the GUI

Screens never call the engine directly — everything goes through `gui/src/engine.rs`, which locks the shared engine, publishes results into the global signals (`gui/src/state.rs`), and toasts on failure:

```rust
// gui/src/engine.rs bridge functions
engine::refresh_sessions();                       // fire-and-forget, generation-guarded
let session = engine::create_session(title, exposition).await?;
engine::play_session(id).await;                   // load + open the last retained turn
engine::prompt(content, from_turn);               // Some(idx) forks at that turn first
engine::goto(index); engine::navigate(delta); engine::jump_to_end();
engine::switch_profile(id).await;
engine::save_config(config).await;
```

## `NovelCraftEngine` Methods (`engine/src/game/engine.rs`)

| Method | Parameters | Returns | Description |
|--------|-----------|---------|-------------|
| `new` | `NovelCraftConfig` | `Self` | Construct the engine (once, at boot — the GUI's `App` boot gate) |
| `set_config` | `NovelCraftConfig` | — | Swap config; re-contributes values into the cached agent loop |
| `set_session` | `Option<SessionV2>` | — | Set the active session; rebuilds agent-loop toolsets from its modules |
| `set_active_profile` | `Option<String>` | — | Update active profile (config + active session) |
| `config` | — | `&NovelCraftConfig` | Read-only config access |
| `profile` | — | `Option<&ProfileV1>` | Resolve the active profile from `config.active_profile` |
| `session` | — | `Result<&SessionV2>` | Active session or error |
| `list_sessions` | — | `Result<Vec<SessionV2>>` | Enumerate saved sessions (metadata only, newest first; skips unreadable/outdated metas with a warning) |
| `create_session` | `title, exposition` | `Result<SessionV2>` | Create + persist a session: default module set, active profile; the exposition doubles as the story premise |
| `prompt` | `content, Sender<AgentMessageChunk>` | `Result<()>` | Push a new user turn and run the DM agent loop |
| `rewrite` | `instruct, Sender<AgentMessageChunk>` | `Result<()>` | Regenerate the last turn under extra steering instructions |
| `fork` | `turn_index` | `Result<()>` | Fork at a retained turn, deleting all later turns |
| `turn` | `index` | `Option<TurnV2>` | Retained turn by window-relative index |
| `turn_count` | — | `Option<usize>` | Retained turn count of the active session |
| `story_state` | — | `Result<StoryState>` | Read long-term story state: rolling summary, chapters |
| `save` | — | `Result<()>` | Persist the active session + config |

## `SessionV2` Methods (`engine/src/game/session.rs`)

| Method | Parameters | Returns | Description |
|--------|-----------|---------|-------------|
| `load` | `id, profiles` | `Result<SessionV2>` | Load meta + snapshot + history; rebuild gamestate via `GameState::replay` |
| `load_metadata` | `id` | `Result<SessionV2>` | Load the session file without rebuilding gamestate (used by `list_sessions`) |
| `delete` | `id` | `Result<()>` | Delete the session file (no-op if absent) |
| `save` | — | `Result<()>` | Write meta, snapshot & history files (`tokio::join!`) |
| `push_turn` | `turn` | `Result<()>` | Append a turn (bumps `total_turns`) and save |
| `update_turn` | `cb: FnOnce(TurnV2) -> Result<TurnV2>` | `Result<()>` | Transform the last turn and save |
| `replace_turn` / `pop_turn` | `turn` / — | `Result<()>` / `Result<Option<TurnV2>>` | Swap / drop the last turn |
| `fork` | `turn_index, profiles` | `Result<()>` | Truncate to `turn_index + 1` (retained window only), rebuild gamestate, save |
| `should_compress` | `threshold` | `bool` | Retained history exceeds `2 × threshold` turns |
| `compress_memory` | `threshold, model` | `Result<bool>` | Summarize + drain the oldest `threshold` turns into the rolling summary; re-anchors the snapshot |
| `turn_count` / `try_turn` | — / `index` | `usize` / `Option<&TurnV2>` | Retained-window accessors |
| `summary` | — | `String` | Rolling story summary read straight from the memory module's snapshot state (no gamestate rebuild; works on metadata-only loads) |

## Gamestate (`engine/src/game/state.rs`)

| Item | Description |
|------|-------------|
| `GameState::new(snapshot, module_ids, profile)` | Build from a snapshot; module IDs missing from the snapshot are seeded with empty state |
| `GameState::replay(modules, turns)` | Re-apply every recorded tool call; errors on unknown module names |
| `GameState::snapshot()` | Export all module states as a `Snapshot` (module ID → JSON object) |
| `GameStateView::get(module, keypath)` | Typed read of a module's JSON state |
| `GameStateView::set(keypath, value)` | Write module state (auto-creates missing keys along the keypath); the only write path, scoped to one module |

## Gameplay Modules (`engine/src/game/module/`)

Each module is a `#[toolset(strip_suffix = "module")] impl AgentToolset<GameStateView>` struct whose `ID` const equals its toolset namespace (snake_case struct name with the `_module` suffix stripped — e.g. `story::manage_timeline — must match what gets recorded in history and replayed).

| Module | ID | Tools | Purpose |
|--------|----|-------|---------|
| `NpcsModule` | `npcs` | NPC CRUD (e.g. `update_apparel`) | NPC persistence (name, personality, physique, apparel) |
| `PlayerModule` | `player` | — | Player-relevant state, initialized from the profile |
| `StoryModule` | `story` | `manage_timeline(contents)`, `begin_chapter(title)`, `note_events(notes)` | Story: `timeline` (TODO list of markdown task items `- [ ]`/`- [x]`; the parser tolerates plain `- ` bullets, normalizing them to unchecked items; updates merge — append new items, check off, ignore uncheck/remove), `summary` (engine-written only), `chapters` (agent-managed; `begin_chapter` titles & closes the active chapter and opens a fresh untitled one, `note_events` appends bullets (duplicates ignored), validated by `BulletList`) |

`StoryModule::read(&GameState)` returns the GUI-facing `StoryState { summary, chapters }`; `StoryModule::context()` injects "Timeline / Story so far / Chapters" into the system prompt. The story premise is injected separately from the session's exposition (`# Story premise` section).

Replay tolerates imperfect history: tool calls whose recorded response is an error are skipped (they had no state effect originally), re-dispatch errors on originally-successful calls are logged and tolerated, and only structurally invalid history (unknown module names, malformed tool names) fails the load.

## Adding a New Engine Capability

1. Add a `pub async fn` (or `pub fn`) on `NovelCraftEngine` — or on `SessionV2`/`GameState` if it belongs at that layer
2. Return `Result<T>` using `EngineError` (`engine/src/error.rs`)
3. Expose it through `gui/src/engine.rs` (lock, call, publish to a `Query` signal, toast on failure)
4. Run `just check`

## Related Documentation

- [Project Structure](./project-structure.md) - File organization and directory layout
- [Data Storage](./database-schema.md) - JSON file formats and storage layout
- [GUI Architecture](./gui-architecture.md) - Dioxus screens, components, and styling conventions
