# Agents Documentation

High-level overview for AI agents working on the NovelCraft project. For comprehensive documentation, see the [detailed docs](./docs/).

**IMPORTANT:** Whenever you make changes, assert validity by running `just check`, then summarize & document the changes with the `@docs-writer` subagent.

**NEVER run `cargo fmt`, `just fmt`, or `just fmt-check` in this project.** rustfmt is not well suited for formatting this codebase's UI code (rsx blocks & long attribute chains) — keep formatting manual.

## Architecture Overview

NovelCraft is a **Rust/Cargo workspace desktop app** built with **Dioxus** (rendered in the system WebView) — fully offline-first, single-user, no server.

- **Engine** (`engine/`, crate `novelcraft-engine`) — pure Rust library crate with all business logic. Owns the game agent loop (built on the `kiruklaw-agent-loop` crate), memory-managed sessions (short-term turn window + long-term memory module), and config/profile persistence. LLM transport (HTTP streaming, SSE parsing) lives inside `kiruklaw-agent-loop`. No UI framework coupling.
- **GUI** (`gui/`, crate `novelcraft-gui`, binary `novelcraft`) — **Dioxus 0.7** app rendered in the platform's system WebView (via `wry`; desktop is primary, Android/iOS are supported compile targets). Components are rsx functions, state is signals, styling is Tailwind CSS v4. Depends on `novelcraft-engine` as a library. Entry point is `gui/src/main.rs`.
- **Filesystem persistence** — all data (session metadata, retained history, state snapshots, config incl. profiles) stored as JSON files under the platform data/config directories (desktop) or the OPFS root via the cfg-split storage seam in `engine/src/util/fs.rs` + `engine/src/util/fs/opfs.rs` (web; paths become logical namespaces, see [Web Target](./docs/web-target.md)). Path resolution is centralized in `engine/src/paths.rs`.
- **No server, no auth, no database** — single-user desktop application with loose JSON files
- **LLM calls** go through the engine — the Dungeon Master agent runs a `kiruklaw` `AgentLoop` whose tool calls operate on per-module gamestate; chunks stream to the GUI through a tokio mpsc channel of `AgentMessageChunk`s. Memory compression uses a ONE-SHOT `kiruklaw_agent_loop::prompt` call with the `Summarizer` model. No event bus or pub/sub.
- **No `AppState`** — the engine is a single `NovelCraftEngine` value (`engine/src/game/engine.rs`) owning the config and the active session. The GUI holds it behind `OnceLock<Arc<Mutex<NovelCraftEngine>>>` (`gui/src/engine.rs`).
- **Build orchestration** is via a root `justfile` — single Cargo workspace (the GUI's `package.json` covers only its Tailwind toolchain)
- **Rust toolchain** pinned to `stable` via `rust-toolchain.toml`

## Quick Reference

### Project Structure

```
novelcraft/
├── justfile                    # Build orchestration (just commands)
├── Cargo.toml                  # Workspace root (members = ["engine", "gui"], resolver = "2")
├── rust-toolchain.toml         # Pins the stable Rust toolchain
├── engine/                     # Rust library crate (novelcraft-engine)
│   ├── Cargo.toml              # Engine dependencies
│   └── src/
│       ├── lib.rs              # Crate root — re-exports modules + kiruklaw AgentMessageChunk
│       ├── config.rs           # NovelCraftConfig, ModelPurpose, Models, DEFAULT_HOST / DEFAULT_SYSTEM_PROMPT / DEFAULT_SUMMARY_THRESHOLD
│       ├── error.rs            # EngineError (thiserror)
│       ├── paths.rs            # data_dir(), config_dir(), sessions_dir() — centralized path resolution
│       ├── util/               # Engine utilities
│       │   ├── mod.rs          # JSON file I/O helpers (serialize, deserialize), timestamp serde, discard_channel
│       │   └── prompting.rs    # PromptFormatter / Promptify — indented prompt text building (with unit tests)
│       ├── markdown/           # Markdown parsing utilities
│       │   ├── mod.rs          # Module barrel (re-exports bullets + todo)
│       │   ├── todo.rs         # TodoItem/TodoList parsing & diffing (with unit tests)
│       │   └── bullets.rs      # BulletList parser — validates chapter notes ("- " per line)
│       └── game/               # Game engine
│           ├── mod.rs          # Module barrel
│           ├── engine.rs       # NovelCraftEngine — prompt/rewrite/fork, create_session, list_sessions, memory
│           ├── session.rs      # SessionV2 — persistence, fork, memory compression (summarize, render_transcript)
│           ├── turns.rs        # TurnV2/AgentResponse + conversation reconstruction
│           ├── state.rs        # GameState / ModuleState / GameStateView — per-module JSON state, replay, snapshots
│           ├── profile.rs      # ProfileV1 — persisted inside config.json
│           └── module/         # Gameplay modules
│               ├── mod.rs      # GameplayModule enum (Story/Npcs/Player/Memory), default_set(), toolset glue
│               ├── story.rs    # story — timeline TODO list management
│           ├── npcs.rs     # npcs — NPC persistence (gamestate-backed; upsert_npc/rename_npc/update_apparel/add_note/remove_note tools, key/value context rendering)
│           ├── player.rs   # player — player-relevant state (context renders only derived apparel; the profile is rendered by the engine)
│               └── memory.rs   # memory — rolling summary/chapters, SUMMARIZER_PERSONA
├── gui/                        # Rust binary crate (novelcraft-gui, binary name: novelcraft)
│   ├── Cargo.toml              # GUI dependencies (dioxus, novelcraft-engine, reqwest, chrono, log, tokio)
│   ├── Dioxus.toml             # dx config (app name, bundle identifier)
│   ├── package.json            # Tailwind toolchain (@tailwindcss/cli, tailwindcss) — bun
│   ├── input.css               # Tailwind source: @theme tokens, base rules
│   ├── assets/tailwind.css     # Compiled stylesheet (build artifact, do not edit)
│   └── src/
│       ├── main.rs             # Entry point — config load, engine construction, BOOT_CONFIG stash, launch, App (stylesheet, toasts, router)
│       ├── engine.rs           # Engine bridge — OnceLock<Arc<Mutex<NovelCraftEngine>>>, async engine ops, generation-guarded query refreshes, turn navigation
│       ├── state.rs            # Global reactive state — Query<T> (Empty/Error/Stale/Recent), GlobalSignals (CONFIG, PROFILES, SESSIONS, SESSION, TURN/TURN_INDEX, STREAM)
│       ├── toast.rs            # Toast notifications (global signal + fixed overlay component)
│       ├── comps.rs            # Shared components (Screen, Button, Chip, Field, FieldGroup, Loading, Markdown, SettingsGear, CloseButton)
│       ├── markdown.rs         # Markdown-like parser for streamed agent output (subset, unit tested)
│       ├── modules.rs          # ModuleContext trait — per-module sidebar rendering & state edits (default tree editor + memory impl)
│       ├── titlebar.rs         # Window chrome — custom title bar (drag/minimize/maximize/close) + resize handles
│       ├── api.rs              # Inspiration REST client (API_BASE_URL, get_json)
│       ├── error.rs            # GuiError (thiserror): Engine/Api variants
│       └── screens/            # Route enum (Routable, mod.rs) + one component module per screen
│           ├── mod.rs          # Route enum + go_back()
│           ├── home.rs         # Session list
│           ├── settings.rs     # Engine config editor (models, system prompt, summary threshold)
│           ├── create_story.rs # New vignette + inspiration browser (debounced tag-filtered fetches)
│           ├── story_overview.rs # Session detail + Play/Delete (with confirm dialog) buttons
│           └── story_gameplay.rs # Game view — turns, streaming, chat bar, memory sidebar, profile popover, swipe
└── docs/                       # Comprehensive documentation
    ├── gui-architecture.md     # Dioxus GUI components, screens, styling conventions
    ├── known-issues.md         # Dev-only issues not worth fixing (check before debugging crashes)
    └── ...
```

---

## Key Conventions Summary

### File Organization

| Category | Location | Pattern |
|----------|----------|---------|
| Engine core | `engine/src/game/engine.rs` | `NovelCraftEngine` — owns config + active session + cached `AgentLoop`; `prompt`/`rewrite`/`fork`/`create_session`/`list_sessions`/`memory` |
| Sessions & memory | `engine/src/game/session.rs` | `SessionV2` — whole-session persistence (one `{id}.json`), `fork`, `compress_memory` |
| Turns | `engine/src/game/turns.rs` | `TurnV2`, `AgentResponse`, `TurnV2::to_conversation` |
| Game state | `engine/src/game/state.rs` | `GameState`/`ModuleState`/`GameStateView` — per-module `RwLock`ed JSON state, replay, snapshots |
| Gameplay modules | `engine/src/game/module/` | `GameplayModule` enum (`module/mod.rs`) + one file per module; each module's `ID` const **must** equal its `#[toolset(strip_suffix = "module")]` namespace |
| Profiles | `engine/src/game/profile.rs` | `ProfileV1` — lives inside `config.json`, not a separate store |
| Engine config | `engine/src/config.rs` | `NovelCraftConfig`, `ModelPurpose`, `Models`, `DEFAULT_HOST`, `DEFAULT_SYSTEM_PROMPT`, `DEFAULT_SUMMARY_THRESHOLD` |
| Path Resolution | `engine/src/paths.rs` | `data_dir()`, `config_dir()`, `sessions_dir()` — single source of truth for filesystem paths |
| Engine Utilities | `engine/src/util/` | `mod.rs` (RFC3339 timestamp serde, `discard_channel`), `fs.rs` (cfg-split file I/O — native `tokio::fs` vs web OPFS), `fs/opfs.rs` (web-only OPFS primitives), `prompting.rs` (`PromptFormatter`, `Promptify`) |
| Markdown Utilities | `engine/src/markdown/` | `todo.rs` (`TodoItem`, `TodoList`, `TodoListDiff`), `bullets.rs` (`BulletList` — chapter-notes validation) |
| GUI Entry | `gui/src/main.rs` | Launch config (desktop-only window settings), `App` component (boot gate: async config load → engine construction → `BOOT_CONFIG` stash → query seeding, then stylesheet link, toast overlay, router) |
| GUI Engine Bridge | `gui/src/engine.rs` | `OnceLock<Arc<Mutex<NovelCraftEngine>>>` — the single seam to the engine; async ops + generation-guarded query refreshes + window-relative turn navigation |
| GUI State | `gui/src/state.rs` | `Query<T>` (Empty/Error/Stale/Recent) + `GlobalSignal`s (`CONFIG`, `PROFILES`, `SESSIONS`, `SESSION`, `TURN`/`TURN_INDEX`, `STREAM`), `Profiles`, `StreamState` |
| GUI Screens | `gui/src/screens/` | `Route` enum (Dioxus Router) + `go_back()` (`mod.rs`) + one component module per screen (`home.rs`, `settings.rs`, `create_story.rs`, `story_overview.rs`, `story_gameplay.rs`) |
| GUI Components | `gui/src/comps.rs` | Shared rsx components (`Screen`, `Button`, `Chip`, `Field`, `FieldGroup`, `Loading`, `SettingsGear`, `CloseButton`), `INPUT_CLASS` |
| GUI Toasts | `gui/src/toast.rs` | `Toast`/`ToastVariant`, `push_toast`/`dismiss_toast`, `Toasts` overlay |
| GUI Styling | `gui/input.css` + `gui/assets/tailwind.css` | Tailwind v4: `@theme` design tokens (`nc-*` colors) → compiled stylesheet (build artifact; `just css` / `just dev`) |
| GUI Text Input | (none — WebView native) | Native `<input>`/`<textarea>`; the platform WebView provides IME, selection & clipboard handling |

### Import Patterns

- **Within engine**: Standard Rust `mod`/`use` paths. No special aliases.
- **GUI depends on engine**: `use novelcraft_engine::game::session::SessionV2;`, `use novelcraft_engine::config::NovelCraftConfig;`, etc. Screens never import engine types directly — they go through `gui/src/engine.rs` and `gui/src/state.rs`.

### Dioxus Navigation & Signals

There is no action system. Navigation goes through the Dioxus Router: `navigator().push(Route::...)`; back is `go_back(from: Route)` (`gui/src/screens/mod.rs`). Always terminate `push(...)` inside event handlers with `;` — non-unit handler returns are interpreted as async spawns and fail to compile. Global signals (`gui/src/state.rs`) must never be touched from `main()` or outside the Dioxus runtime; read with `.read()` (clone out of the guard for `'static` closures), write globals via `*X.write() = ...` and locals via `x.set(...)`/`x.write()` (locals need `let mut`). See [GUI Architecture](./docs/gui-architecture.md) for details.

### AI / Model Configuration

Models are configured in Rust, persisted to disk as JSON, and held in the engine config.

- **Engine side**: `NovelCraftConfig` (defined in `engine/src/config.rs`, persisted at `{configDir}/NovelCraft/config.json`) holds `max_agent_steps`, `system_prompt`, `models: Models` — an `exhaustive_map::ExhaustiveMap<ModelPurpose, ModelConfig>` keyed by **all three** `ModelPurpose` variants (`DungeonMaster`, `Suggestions`, `Summarizer`) — plus `summary_threshold`, `profiles`, and `active_profile`.
- Each `ModelConfig` (re-exported from `kiruklaw-agent-loop`, internally tagged) is `ModelConfig::OpenAi { base_url, api_key, model }`, where `model` is the actual LLM API model identifier.
- **Because `Models` is exhaustive, `config.json` must contain a `summarizer` key.** The `Summarizer` purpose was added in the memory rewrite — existing users migrate their local config manually. `summary_threshold` is `#[serde(default)]` (defaults to `DEFAULT_SUMMARY_THRESHOLD` = 50), so older configs still deserialize.
- `NovelCraftConfig::contribute()` pushes `max_agent_steps`, `system_prompt` (the DM persona), and the Dungeon Master model into the `AgentLoop`. The loop is also built with `hide_tool_history` enabled: tool calls & results from earlier rounds are stripped from each request (module contexts and the rolling summary re-inject the relevant state; the narrative text reflects their outcomes) — a token-usage optimization. It also requests reasoning (`reasoning: { effort: "medium" }`, the OpenRouter-unified parameter; models without reasoning support ignore it) so the DM plans its tool calls before narrating. The Summarizer model is used only by the one-shot memory-compression call, without reasoning.
- Default host is `http://localhost:8888/v1` (`DEFAULT_HOST` — local llama.cpp/LM Studio server); the default system prompt is the Dungeon Master persona in `config.rs` (it instructs the DM to maintain chapter notes with the memory tools).
- The GUI's Settings screen edits this config (three model groups + "Summary Threshold (turns)" field) and persists it via `engine::save_config` (`config.save()` + `NovelCraftEngine::set_config`).

### Agent / LLM Integration

**All LLM calls go through the engine.** HTTP transport and SSE parsing live in `kiruklaw-agent-loop`.

- **Dungeon Master agent loop** — `NovelCraftEngine::prompt(content, sender)` / `rewrite(instruct, sender)` (`engine/src/game/engine.rs`):
  1. Refresh the module-context cache (a module's `context()` is re-rendered only when its state is dirty or uncached).
  2. Push a new turn (`prompt`) or attach steering instructions to the last turn and clear its responses (`rewrite`).
  3. Assemble the conversation: a single system message (config persona + `# Gameplay module contexts` + player character profile) followed by the retained short-term history, then the `TOOL_REMINDER` system message spliced in immediately before the latest user prompt (`inject_tool_reminder`) — nudging the agent to document the turn via its tools.
  4. Run the `kiruklaw` `AgentLoop` with a `ContextProvider` that hands each toolset a `GameStateView` scoped to its module.
  5. Assimilate the newly produced assistant/tool messages into the turn (`TurnV2::assimilate`) and commit.
  6. Run `compress_memory_if_due()` (see [Session & Memory Lifecycle](#session--memory-lifecycle)).
- **Streaming** — chunks arrive as `AgentMessageChunk` variants (`Content { text }`, `ToolCallStart`, `ToolCallArgs`, `Reasoning`, `Done`) through a tokio mpsc `Sender` that the caller provides.
- **Memory compression** — a ONE-SHOT `kiruklaw_agent_loop::prompt` call (no agent loop, no tools) using the `Summarizer` model and `SUMMARIZER_PERSONA`.

**Important terminology:** A "persona" is ONLY the system prompt passed to the LLM call — it defines who the agent *is*. Exactly two personas exist: the DM persona (`NovelCraftConfig.system_prompt`, default `DEFAULT_SYSTEM_PROMPT` in `config.rs`) and `SUMMARIZER_PERSONA` (`engine/src/game/module/story.rs`). Everything else — turn-level `system` fields (DM steering instructions for rewrites), module contexts, the player profile — is **NOT** a persona. They are regular messages (or parts of the system message) injected into the conversation to guide behavior.

### Data Persistence — Filesystem via Engine Types

All data is stored as **JSON files** on disk, managed by engine types (`SessionV2`, `NovelCraftConfig`).

- **File layout** (roots resolved by `engine/src/paths.rs`, which appends `NovelCraft` to the platform directories):
  - `{dataDir}/NovelCraft/sessions/{sessionUUID}.json` — the ENTIRE session in one file (`SessionV2`: metadata incl. `total_turns`, the gamestate `snapshot` anchoring the retained turns, and the retained `turns`)
  - `{configDir}/NovelCraft/config.json` — `NovelCraftConfig` incl. all profiles and the active profile ID
  - Those two files per session/config are ALL there is — no page batches, no checkpoint files, no separate profiles/stories/lore stores.
- **Version-gated deserialization**: the session file carries a `version` field validated by a custom deserializer — `SessionV2` requires version `2`; anything else is rejected (legacy v1 multi-file session *directories* are ignored; `list_sessions` warns about them). `summary_threshold` in the config is `#[serde(default)]` for forward compatibility.
- **One file, one source of truth**: `SessionV2::save` writes the whole session atomically enough (single file write) — restoring state means reading one file and rebuilding the gamestate from `snapshot` + replaying `turns`. The flat-file layout is also the stepping stone for a future binary format (swap the serde impl, keep the struct).

### Session & Memory Lifecycle

The conversation is the agent's **short-term memory**: only the last `2 × summary_threshold` turns are retained (default threshold 50). Everything older is compressed into a rolling story summary held by the memory module (**long-term memory**).

- **Core invariant** — the session snapshot holds the gamestate as it was just BEFORE the first retained turn. Loading = `GameState::new(snapshot, module_ids, profile)` + `GameState::replay(history)`: error'ed tool calls are **skipped** (their recorded response is an error — they had no state effect originally, and re-dispatching could double-apply if validation semantics changed); originally-successful calls are re-dispatched, and a re-dispatch error (state drift) is logged, never fatal. After each turn's tool calls, that turn's manual edits (`TurnV2::edits` — sidebar overrides, keypath[0] = module ID, empty string removes) are applied; bad edits (unknown module, unwritable path) are logged and skipped. Only structurally invalid history errors (unknown module names, malformed tool names).
- **Session creation** — `NovelCraftEngine::create_session(title, exposition)`: builds a fresh `SessionV2` via `SessionV2::default()` (modules filled by `GameplayModule::default_set()`: `story`, `npcs`, `player`), records the engine's active profile, and persists. The exposition doubles as the story premise: it is injected into the system prompt from the session (see below), never copied into gamestate.
- **Module ID invariant** — module IDs (`story`, `npcs`, `player`) MUST match the toolset namespace. The kiruklaw `#[toolset]` macro derives the namespace from the snake_case struct name, and every module declares `#[toolset(strip_suffix = "module")]` to strip the `_module` suffix (`StoryModule` → `story`). `GameState::replay` resolves recorded tool calls by that namespace against the session's module map. Mismatches silently break replay.
- **Compression** — after each prompt/rewrite commit, `NovelCraftEngine::compress_memory_if_due()` → `SessionV2::compress_memory(threshold, model)` when `should_compress()` (retained len > 2 × threshold):
  1. Render the oldest `threshold` turns into a transcript (`render_transcript`) prefixed by the previous rolling summary, if any.
  2. ONE-SHOT Summarizer LLM call merges the transcript into the rolling summary. **Summarize FIRST**: on LLM failure nothing is mutated; the engine logs a warning and compression silently retries on a later trigger — the player's turn never fails.
  3. Drain the old turns, write the summary into the gamestate (`StoryModule::write_summary` — engine-written; agents never touch it), re-snapshot the gamestate (re-anchoring it to the new first retained turn), save.
- **Fork** — `NovelCraftEngine::fork(turn_index)` → `SessionV2::fork(turn_index, profiles)`: works ONLY within the retained window (errors otherwise); truncates history to `turn_index + 1`, rebuilds the gamestate from snapshot + replay, saves. In the GUI, sending a prompt from any turn other than the last forks at that turn first (`engine::prompt(content, from_turn)`).
- **Long-term memory (story module)** — gamestate keys under `story`: `timeline` (agent-managed TODO list of markdown task items `- [ ]`/`- [x]`; the parser tolerates plain `- ` bullets and normalizes them to unchecked items; `manage_timeline` **merges** rather than replaces — new items append at the end, checked state only moves forward, uncheck/remove attempts are ignored), `summary` (rolling summary, engine-written only) and `chapters: Vec<ChapterV2>` (`{title, notes}` — notes are markdown bullet lists). Chapters bootstrap lazily (there is always an active, untitled last chapter). Agents maintain them via `note_events(notes)` (appends bullets to the active chapter's notes — duplicates ignored, the record never shrinks; validated by the `BulletList` parser — every non-empty line must start with `"- "`) and `begin_chapter(title)` (closes the active chapter under the given descriptive title and opens a fresh untitled one). The story premise is NOT gamestate — it comes from the session's `exposition` and is injected into the system prompt (`# Story premise` section in `NovelCraftEngine::system_prompt_msg`). `StoryModule::read(&GameState)` returns the GUI-facing `StoryState { summary, chapters }`; the engine exposes it as `NovelCraftEngine::story_state()`. `StoryModule::context()` injects "Timeline / Story so far / Chapters" into the system prompt via the module-context cache.

### Game Agent Loop

The `kiruklaw` `AgentLoop` iterates LLM calls until a final text response is received or tool calls are resolved (bounded by `max_agent_steps` from the config).

- Tools are registered per session from its gameplay modules (`build_agent_loop` → `with_toolsets`); tool names are namespaced `<module_id>::<tool>`.
- Tool calls execute against the shared `GameState` through a per-module `GameStateView` (the only type allowed to write module state).
- The loop's transcript is appended to the in-flight conversation; after it finishes, `TurnV2::assimilate` folds the assistant + tool messages into the session's last turn and the turn is committed.

---

## Common Tasks

### Running the Application

All commands are managed by the root `justfile`. Run `just` with no arguments to list available recipes.

```bash
# Run the app with hot-reload (Tailwind watcher + dx serve --platform desktop)
just dev

# Bundle the app for the current platform
just build

# Compile the Tailwind stylesheet once
just css

# Check (cargo check)
just check

# Per-crate checking
just check-engine
just check-gui
```

### Type Checking & Linting

```bash
# Cargo check (entire workspace)
just check

# Cargo clippy
just clippy
```

Do not run `just fmt` / `just fmt-check` — see the note at the top of this file.

---

## Data Access Patterns

### GUI → Bridge → Engine

The GUI never touches files or engine internals directly: screens call `gui/src/engine.rs` bridge functions, which lock the shared engine and publish results into the global signals.

```rust
// GUI side (inside a component/task):
engine::refresh_sessions();                              // fire-and-forget, generation-guarded
let session = engine::create_session(title, exposition).await?;
engine::play_session(id).await;                          // loads + opens the last retained turn
engine::prompt(content, from_turn);                      // Some(idx) forks at that turn first
engine::goto(idx); engine::navigate(delta); engine::jump_to_end();
engine::delete_session(id).await?;                       // deletes from disk + clears active session if it was
```

```rust
// Engine side (plain Rust):
use novelcraft_engine::game::engine::NovelCraftEngine;
use novelcraft_engine::game::session::SessionV2;

let sessions = NovelCraftEngine::list_sessions().await?; // metadata-only, newest first
let mut session = SessionV2::load(id, &profiles).await?;
session.fork(turn_index, &profiles).await?;
session.save().await?;
```

**No transactions**: each save writes whole files; sequential operations are driven by the caller.

---

## Engine API Reference

### `NovelCraftEngine` (`engine/src/game/engine.rs`)

| Function | Parameters | Returns | Description |
|----------|-----------|---------|-------------|
| `new` | `NovelCraftConfig` | `Self` | Construct the engine (once, at boot — the GUI's `App` boot gate) |
| `set_config` | `NovelCraftConfig` | — | Swap config; re-contributes values into the cached agent loop |
| `set_session` | `Option<SessionV2>` | — | Set the active session; rebuilds agent loop toolsets from its modules |
| `set_active_profile` | `Option<String>` | — | Update active profile (config + active session) |
| `create_profile` | — | `Result<ProfileV1>` | Append a new profile (seeded with Gender/Age/Personality/Appearance), make it the active one, persist |
| `update_profile` | `ProfileV1` | `Result<()>` | Replace the profile (matched by id) and persist; refreshes the active session's copy if it is the active profile |
| `config` | — | `&NovelCraftConfig` | Read-only config access |
| `profile` | — | `Option<&ProfileV1>` | Resolve the active profile |
| `session` | — | `Result<&SessionV2>` | Active session or error |
| `list_sessions` | — | `Result<Vec<SessionV2>>` | Enumerate saved sessions (metadata only, newest first; skips unreadable/outdated metas) |
| `create_session` | `title, exposition` | `Result<SessionV2>` | Create + persist a session with module defaults; the exposition doubles as the story premise |
| `prompt` | `content, Sender<AgentMessageChunk>` | `Result<()>` | Push a new user turn and run the DM agent loop |
| `rewrite` | `instruct, Sender<AgentMessageChunk>` | `Result<()>` | Regenerate the last turn under the given steering instructions |
| `fork` | `turn_index` | `Result<()>` | Fork at a retained turn, deleting all later turns |
| `reset_history` | — | `Result<()>` | Drop the entire retained history, restoring the gamestate to the session snapshot (fresh start of the story) |
| `regenerate` | `Sender<AgentMessageChunk>` | `Result<()>` | Regenerate the last response: fork from the second-to-last turn (drop the entire history for the very first turn) and re-run the same user prompt |
| `set_last_prompt` | `content` | `Result<()>` | Overwrite the user prompt of the last retained turn (GUI inline editing, saved on blur) |
| `turn` | `index` | `Option<TurnV2>` | Get a retained turn by window-relative index |
| `turn_count` | — | `Option<usize>` | Retained turn count of the active session |
| `story_state` | — | `Result<StoryState>` | Read the long-term story state (summary, chapters) |
| `effective_prompt` | — | `Result<String>` | The effective prompt the DM agent loop sees next, rendered for display: system message (persona, premise, module contexts, profile) + retained conversation history, **mirroring `hide_tool_history`** (old tool calls & results stripped when the loop hides them); refreshes the module-context cache |
| `save` | — | `Result<()>` | Persist the active session + config |

### `SessionV2` (`engine/src/game/session.rs`)

| Function | Parameters | Returns | Description |
|----------|-----------|---------|-------------|
| `load` | `id, profiles` | `Result<SessionV2>` | Load meta + snapshot + history, rebuild gamestate via replay |
| `load_metadata` | `id` | `Result<SessionV2>` | Load the session file without rebuilding gamestate (used by `list_sessions`) |
| `delete` | `id` | `Result<()>` | Delete the session file (no-op if absent) |
| `save` | — | `Result<()>` | Write meta, snapshot & history files |
| `push_turn` | `turn` | `Result<()>` | Append a turn (bumps `total_turns`) and save |
| `update_turn` | `cb: FnOnce(TurnV2) -> Result<TurnV2>` | `Result<()>` | Transform the last turn and save |
| `replace_turn` / `pop_turn` | `turn` / — | `Result<()>` / `Result<Option<TurnV2>>` | Swap / drop the last turn |
| `fork` | `turn_index, profiles` | `Result<()>` | Truncate to `turn_index + 1` (retained window only), rebuild gamestate, save |
| `reset_turns` | `profiles` | `Result<()>` | Drop all retained turns and restore the gamestate to the snapshot (used when regenerating the very first turn) |
| `should_compress` | `threshold` | `bool` | Retained history exceeds `2 × threshold` |
| `compress_memory` | `threshold, model` | `Result<bool>` | Summarize + drain the oldest `threshold` turns (see lifecycle) |
| `turn_count` / `try_turn` | — / `index` | `usize` / `Option<&TurnV2>` | Retained-window accessors |
| `summary` | — | `String` | Rolling story summary from the memory module's snapshot state (no gamestate rebuild; works on metadata-only loads) |

### `GameState` / module state (`engine/src/game/state.rs`)

| Function | Parameters | Returns | Description |
|----------|-----------|---------|-------------|
| `GameState::new` | `snapshot, module_ids, profile` | `Result<GameState>` | Build from snapshot; module IDs missing from the snapshot are seeded with empty state |
| `GameState::replay` | `modules, turns` | `Result<()>` | Re-apply recorded tool calls; skips error'ed calls (recorded `Error:` responses), tolerates re-dispatch drift, errors on unknown module names |
| `GameState::snapshot` | — | `Snapshot` | Export all module states |
| `GameStateView::get` / `set` | `module, keypath` / `keypath, value` | … / `Result<()>` | Typed read / write of a module's JSON state (writes auto-create missing keys along the keypath) |

---

## Agent Guidelines

### When to Add an Engine Capability

- The GUI needs a new operation on the session, config, memory, or gamestate.

**Process:**
1. Add a `pub async fn` (or `pub fn`) on `NovelCraftEngine` — or on `SessionV2`/`GameState` if it belongs at that layer
2. Return `Result<T>` using the engine's error type (`EngineError`)
3. Expose it through `gui/src/engine.rs` (lock, call, publish to a `Query` signal, toast on failure) — never reach into the engine from screens
4. Run `just check`

### When to Modify Data Formats

- Entity structures change (new fields, renamed fields) or new persisted types appear.

**Process:**
1. Modify the Rust structs in `engine/src/game/` (`session.rs`, `turns.rs`, `config.rs`)
2. Backward-compatible additions: use `#[serde(default)]` (see `summary_threshold`, `TurnV2::responses`)
3. Backward-incompatible changes: bump the type's `VERSION` const and extend its `deserialize_version` gate — old files are rejected, not misread (this is how v1 sessions were retired)
4. Remember `Models` is exhaustive: any new `ModelPurpose` variant makes existing `config.json` files invalid until the user adds the key

### When to Add a GUI Component

- Reusable UI patterns (rsx components)
- Complex rendering that should be isolated into its own `#[component]`

**Process:**
1. Create a new `#[component] fn ... -> Element` in `gui/src/comps.rs` (shared) or the relevant screen module
2. Compose the tree with `rsx!` and style with Tailwind classes (`@theme` tokens for colors — add new tokens to `gui/input.css` if needed)
3. For new data access, add the operation to `gui/src/engine.rs` and a `Query` signal to `gui/src/state.rs` — never reach into the engine from screens

### When to Add a Game Module

- New gameplay mechanics that require LLM tool access
- New state that needs snapshot/replay/fork support

**Process:**
1. Define the module struct with a `#[toolset(strip_suffix = "module")] impl AgentToolset<GameStateView>` and a `pub const ID: &str` that **equals the struct's snake_case name with the `_module` suffix stripped** (the tool namespace must match what gets recorded in history and replayed)
2. Add a `GameplayModule` variant in `engine/src/game/module/mod.rs` plus its `default_set()`, `as_toolset`, `id`, `context` (and `init` if needed) match arms
3. Implement `context()` so the module's state is injected into the system prompt (cached; re-rendered when the module state is dirty)
4. Ensure state is JSON-object-serializable (`GameState` stores each module's state as a JSON object)

---

## Justfile Commands

All build and development commands are in the root `justfile`.

| Recipe | Purpose |
|--------|---------|
| `just dev` | Run the app with hot-reload (Tailwind watcher + `dx serve --platform desktop`) |
| `just css` | Compile the Tailwind stylesheet once (minified) |
| `just build` | Bundle the app for the current platform (`just css` + `dx bundle --platform desktop`) |
| `just build-web` | Build the web (wasm32) bundle (`just css` + `dx build --platform web`) |
| `just llama` | Serve a local GGUF model on `:8888` via llama.cpp (`DEFAULT_HOST`) |
| `just check` | Cargo check (entire workspace) |
| `just check-engine` | Cargo check (`novelcraft-engine` only) |
| `just check-gui` | Cargo check (`novelcraft-gui` only) for desktop AND web targets (the wasm pass negates default features, enabling `dioxus/web` — the desktop renderer cannot target wasm) |
| `just clippy` | Cargo clippy (entire workspace) |
| `just fmt` | Cargo fmt (entire workspace) — **do not use**, see note at top |
| `just fmt-check` | Cargo fmt --check (entire workspace) — **do not use**, see note at top |

## Key Dependencies

### Engine (engine/Cargo.toml, crate: novelcraft-engine)

| Crate | Purpose |
|-------|---------|
| `kiruklaw-agent-loop` | Agent loop, toolsets (`#[toolset]`), LLM HTTP streaming + SSE parsing, `ModelConfig` (git dependency) |
| `tokio` (full) | Async runtime |
| `serde` / `serde_json` | Serialization — gamestate is stored as JSON values |
| `exhaustive-map` | `Models` as `ExhaustiveMap<ModelPurpose, ModelConfig>` |
| `dirs` | Platform directory resolution (`data_dir`, `config_dir`) |
| `uuid` (v4) | Session IDs |
| `chrono` | Timestamps (RFC3339 in files) |
| `thiserror` / `anyhow` | `EngineError` / tool-handler error types |
| `log` / `env_logger` | Logging (compression failures, skipped sessions) |
| `itertools` | Small formatting helpers (e.g. joining tool names) |

### GUI (gui/Cargo.toml, crate: novelcraft-gui, binary: novelcraft)

| Crate | Purpose |
|-------|---------|
| `novelcraft-engine` (path) | Business logic library |
| `dioxus` (features: desktop, router) | UI framework — rsx components, signals, router; renders in the system WebView via `wry` |
| `@tailwindcss/cli` + `tailwindcss` (npm, via bun — see `gui/package.json`) | Tailwind CSS v4 toolchain; compiles `gui/input.css` → `gui/assets/tailwind.css` |
| `reqwest` (version 0.12, json feature) | Inspiration REST client (streaming not used; json feature) |
| `log` | Logging facade (explicit imports — a glob `use log::*` collides with the Dioxus prelude) |
| `chrono` | Local-time formatting of session timestamps (used by `screens/home.rs`) |
