# Project Structure

This document outlines the complete folder organization and directory layout for NovelCraft.

NovelCraft is a Rust/Cargo workspace split into two crates: **novelcraft-engine** (pure Rust library) and **novelcraft-gui** (Dioxus-based binary). Licensed under GPL-3.0. Offline-first, single-user, no server, no database.

## Directory Tree

```
novelcraft/
├── justfile                          # Build orchestration (just commands)
├── Cargo.toml                        # Workspace root (members = ["engine", "gui"], resolver = "2")
├── rust-toolchain.toml               # Pins the stable Rust toolchain
├── AGENTS.md                         # High-level overview for AI agents
├── README.md                         # Project overview, prerequisites, build instructions
├── engine/                           # Rust library crate (novelcraft-engine)
│   ├── Cargo.toml                    # Engine dependencies
│   └── src/
│       ├── lib.rs                    # Crate root — module declarations + kiruklaw re-exports (agent_loop, AgentMessageChunk)
│       ├── config.rs                 # NovelCraftConfig, ModelPurpose, Models, DEFAULT_HOST / DEFAULT_SYSTEM_PROMPT / DEFAULT_SUMMARY_THRESHOLD
│       ├── error.rs                  # EngineError (thiserror)
│       ├── paths.rs                  # data_dir(), config_dir(), sessions_dir() — centralized path resolution
│       ├── rustfmt.toml              # tab_spaces = 2 (formatting is manual — do not run rustfmt)
│       ├── util/                     # Engine utilities
│       │   ├── mod.rs                # JSON file I/O helpers (serialize, deserialize), RFC3339 timestamp serde, discard_channel
│       │   └── prompting.rs          # PromptFormatter / Promptify — indented prompt text building (with unit tests)
│       ├── markdown/                 # Markdown parsing utilities
│       │   ├── mod.rs                # Module barrel (re-exports bullets + todo)
│       │   ├── todo.rs               # TodoItem / TodoList / TodoListDiff — timeline parsing & diffing
│       │   └── bullets.rs            # BulletList parser — validates chapter notes ("- " per line)
│       └── game/                     # Game engine
│           ├── mod.rs                # Module barrel
│           ├── engine.rs             # NovelCraftEngine — prompt/rewrite/fork, create_session, list_sessions, story_state/module_state, module-context cache
│           ├── session.rs            # SessionV2 — persistence (meta/snapshot/history), fork, memory compression (render_transcript, summarize)
│           ├── turns.rs              # TurnV2 / AgentResponse + conversation reconstruction
│           ├── state.rs              # GameState / ModuleState / GameStateView — per-module JSON state, replay, snapshots
│           ├── profile.rs            # ProfileV1 — persisted inside config.json
│           └── module/               # Gameplay modules
│               ├── mod.rs            # GameplayModule enum (Story/Npcs/Player/Memory), default_set(), toolset/context glue
│               ├── story.rs          # story — timeline TODO list (manage_timeline tool)
│               ├── npcs.rs           # npcs — NPC persistence
│               ├── locations.rs      # locations — location persistence (planned: traveling & navigation)
│               ├── player.rs         # player — player-relevant state
│               └── story.rs          # story — timeline TODO list, rolling summary / chapters, SUMMARIZER_PERSONA
├── gui/                              # Rust binary crate (novelcraft-gui, binary name: novelcraft)
│   ├── Cargo.toml                    # GUI dependencies (dioxus, novelcraft-engine, reqwest, chrono, log, tokio)
│   ├── Dioxus.toml                   # dx config (app name, bundle identifier)
│   ├── package.json                  # Tailwind toolchain (@tailwindcss/cli, tailwindcss) — bun
│   ├── input.css                     # Tailwind source: @theme tokens, base rules
│   ├── assets/tailwind.css           # Compiled stylesheet (build artifact, do not edit)
│   └── src/
│       ├── main.rs                   # Entry point — config load, engine construction, BOOT_CONFIG stash, launch, App
│       ├── engine.rs                 # Engine bridge — OnceLock<Arc<Mutex<NovelCraftEngine>>>, async engine ops, generation-guarded refreshes, window-relative turn navigation
│       ├── state.rs                  # Global reactive state — Query<T>, GlobalSignals (CONFIG, PROFILES, SESSIONS, SESSION, TURN/TURN_INDEX, STREAM)
│       ├── toast.rs                  # Toast notifications (global signal + overlay component)
│       ├── comps.rs                  # Shared components (Screen, Button, Chip, Field, ...)
│       ├── titlebar.rs               # Window chrome — custom title bar (drag/min/max/close) + resize handles
│       ├── api.rs                    # Inspiration REST client (API_BASE_URL, get_json)
│       ├── error.rs                  # GuiError (Engine/Api variants)
│       └── screens/
│           ├── mod.rs                # Route enum (Routable) + go_back()
│           ├── home.rs               # Session list
│           ├── settings.rs           # Engine config editor (three model groups, system prompt, summary threshold)
│           ├── create_story.rs       # New vignette + inspiration browser (debounced tag-filtered fetches)
│           ├── story_overview.rs     # Session detail (title/exposition) + Play/Delete buttons
│           └── story_gameplay.rs     # Game view — turn cards, streaming, chat bar, module sidebar, profile popover, swipe
└── docs/                             # Project documentation
    ├── project-structure.md          # This file
    ├── code-conventions.md           # Code styling and conventions
    ├── database-schema.md            # JSON file formats and storage layout
    ├── api-routes.md                 # Engine API reference (NovelCraftEngine methods)
    ├── gui-architecture.md           # Dioxus GUI components, screens, styling conventions
    └── frontend-architecture.md      # [vestigial] Vue/Tauri frontend (no longer used)
```

## Key Directories

### `engine/` — Rust Library Crate (`novelcraft-engine`)

Pure Rust library with all business logic. No UI framework coupling.

**`engine/src/`**
- `lib.rs` — Crate root; declares the modules and re-exports `kiruklaw_agent_loop` as `agent_loop` plus `AgentMessageChunk`
- `config.rs` — `NovelCraftConfig` (`max_agent_steps`, `system_prompt`, `models`, `summary_threshold`, `profiles`, `active_profile`). `ModelPurpose` enum (`DungeonMaster`/`Suggestions`/`Summarizer`); `Models` is an `ExhaustiveMap<ModelPurpose, ModelConfig>` — config.json must contain all three keys. Persisted at `{configDir}/NovelCraft/config.json`. `contribute()` pushes max steps, persona & DM model into the agent loop
- `error.rs` — `EngineError` (thiserror) with constructors like `EngineError::state`, `EngineError::validation`, `EngineError::module_not_found`
- `paths.rs` — `data_dir()`, `config_dir()`, `sessions_dir()` — append `NovelCraft` to the platform directories (`dirs` crate); single source of truth for filesystem paths
- `util/` — `mod.rs`: RFC3339 timestamp serde, `discard_channel`. `fs.rs`: cfg-split file I/O (`serialize`, `deserialize`, `ensure_dir`, `remove_file`, `list_dir` — native `tokio::fs` vs web OPFS), `fs/opfs.rs`: web-only OPFS primitives. `prompting.rs`: `PromptFormatter`/`Promptify` for building indented prompt text (used for module contexts, transcripts, summaries)
- `markdown/` — `todo.rs`: `TodoList`/`TodoItem`/`TodoListDiff` (story timeline). `bullets.rs`: `BulletList` — every non-empty line must start with `"- "`; validates memory-chapter notes
- `game/` — the game engine:
  - `engine.rs` — `NovelCraftEngine`: owns config + active `SessionV2` + cached `AgentLoop`. `prompt`/`rewrite` run the DM agent loop (system message = persona + module contexts + player profile, then retained history); `fork`; `regenerate` (fork from the second-to-last turn — or reset the history for the very first — and re-run the same prompt); `create_session`; `list_sessions` (metadata only, newest first); `memory()` read model; `compress_memory_if_due` after every commit
  - `session.rs` — `SessionV2`: memory-managed session persisted as ONE file `{sessionsDir}/{id}.json` (file version gate `== 2`; carries metadata incl. `total_turns`, the gamestate `snapshot`, and the retained `turns`). `fork(turn_index, profiles)` only within the retained window; `reset_turns(profiles)` drops the whole history (regenerating the very first turn); `compress_memory(threshold, model)` summarizes the oldest `threshold` turns into the rolling summary (one-shot `kiruklaw_agent_loop::prompt` with `SUMMARIZER_PERSONA`; nothing is mutated on LLM failure)
  - `turns.rs` — `TurnV2` (`system`/`prompt`/`responses`), `AgentResponse` (content + tool calls + tool responses), `TurnV2::to_conversation` (rebuilds the agent-loop conversation from retained turns); `Snapshot` (module ID → JSON state map) lives in `state.rs`
  - `state.rs` — `GameState` (per-module `RwLock`ed JSON state + dirty flags), `ModuleState::set` (auto-creates missing keys along the keypath), `GameState::replay` (re-applies recorded tool calls; errors on unknown modules), `GameStateView` (the only writer, scoped to one module)
  - `profile.rs` — `ProfileV1` (`id`, `name`, `fields`, timestamps; `Promptify` impl for system-prompt injection)
  - `module/` — `GameplayModule` enum + one file per module. IDs (`story`, `npcs`, `locations`, `player`) **must** equal the module's toolset namespace — the kiruklaw `#[toolset]` macro derives it from the snake_case struct name, with `#[toolset(strip_suffix = "module")]` stripping the `_module` suffix, and replay resolves recorded calls by it. `story.rs` also defines `SUMMARIZER_PERSONA`, `StoryState`/`ChapterV2`, and the `manage_timeline`/`begin_chapter`/`note_events` tools

### `gui/` — Rust Binary Crate (`novelcraft-gui`, binary name: `novelcraft`)

Dioxus 0.7 GUI rendered in the system WebView (via `wry`); styled with Tailwind CSS v4. Depends on `novelcraft-engine` as a library. Desktop is the primary target; Android/iOS are supported compile targets.

**`gui/src/`**
- `main.rs` — Binary entry point — loads `NovelCraftConfig` (falls back to defaults), constructs the engine (`engine::init_engine`), stashes `BOOT_CONFIG`, `dioxus::LaunchBuilder` + undecorated window config, `App` component (stylesheet link, title bar, router, toast overlay)
- `engine.rs` — Engine bridge — `OnceLock<Arc<Mutex<NovelCraftEngine>>>`; async engine operations (`play_session`, `create_session`, `switch_profile`, `save_config`, `effective_prompt_async`, `module_state_async`/`set_module_state_async`, `set_last_prompt_async`, `prompt`/`regenerate` streaming), generation-guarded query refreshes, window-relative turn navigation (`load_turn`/`goto`/`navigate`/`jump_to_end`)
- `state.rs` — Global reactive state — `Query<T>` (Empty/Error/Stale/Recent), `GlobalSignal`s (`CONFIG`, `PROFILES`, `SESSIONS`, `SESSION`, `TURN`/`TURN_INDEX`, `STREAM`), `Profiles`/`StreamState` types
- `toast.rs` — `Toast`/`ToastVariant`, `push_toast`/`dismiss_toast`, `Toasts` overlay component
- `comps.rs` — Shared components (`Screen`, `Button`, `Chip`, `Field`, `FieldGroup`, `Loading`, `Markdown`, `SettingsGear`, `CloseButton`), `INPUT_CLASS` constant
- `modules.rs` — `ModuleContext` trait for per-module sidebar rendering & state edits: `module_context(id)` resolves custom impls (memory) with a generic tree-editor fallback; sidebar iterates the session's active modules. `set_path` handles object keys & decimal array indices
- `markdown.rs` — Markdown-like parser tuned for streaming agent output (`parse` → `Vec<Block>`; inline `Segment` tree — plain/bold/italic runs, and double-quoted `"speech"` as a container segment nesting emphasis so a quote renders as one block; unterminated constructs render as-if closed). Subset: paragraphs, `#` headings, `>` blockquotes, `**bold**`, `*italic*`, `"speech"` (own line, white). Unit tested
- `titlebar.rs` — Custom window chrome (`TitleBar`, `ResizeHandles`) for the undecorated window
- `api.rs` — Inspiration REST client (`API_BASE_URL`, `get_json`)
- `error.rs` — `GuiError` (via `thiserror`): `Engine` (wraps `EngineError`) and `Api` (message string) variants
- `screens/` — `Route` enum (`mod.rs`, Dioxus Router) + one component module per screen
  - `mod.rs` — `Route` enum, `go_back()`
  - `home.rs` — Home screen (session launcher; cards render the rolling story summary — falling back to the premise — via `Markdown`)
  - `settings.rs` — Settings screen (Max Agent Steps, Summary Threshold (turns), System Prompt, three model groups: Dungeon Master / Suggestions / Summarizer)
  - `create_story.rs` — Vignette creation form (title + premise + optional creative direction, appended to the DM persona for this story) + Get Inspired section (debounced tag-filtered fetches); submits via `engine::create_session`
  - `story_overview.rs` — Session overview (banner placeholder, title/exposition, Play + Delete buttons; Delete opens a screen-local `ConfirmDialog` modal, then `engine::delete_session` removes the file and clears the engine's active session if it was the one deleted)
  - `story_gameplay.rs` — Gameplay screen (turn cards with swipe, streaming cards, ☰ sidebar — premise, effective prompt, one collapsible section per active module via `ModuleContext`, tool calls of the viewed turn — chat bar, status row, profile popover & dialog)

**`gui/` (non-Rust)**
- `Dioxus.toml` — dx app config (name, bundle identifier)
- `input.css` — Tailwind v4 source (`@import "tailwindcss"`, `@theme` design tokens)
- `assets/tailwind.css` — compiled stylesheet (build artifact; regenerate with `just css`)
- `package.json` / `bun.lock` — Tailwind CLI toolchain (`@tailwindcss/cli`, `tailwindcss`)

### `docs/` — Documentation

Comprehensive project documentation for developers and contributors.

## Build Orchestration

The root `justfile` is the single source of truth for all build commands.

| Recipe | Purpose |
|--------|----------|
| `just dev` | Run the app with hot-reload (Tailwind watcher + `dx serve --platform desktop`) |
| `just css` | Compile the Tailwind stylesheet once (minified) |
| `just build` | Bundle the app for the current platform (`just css` + `dx bundle --platform desktop`) |
| `just llama` | Serve a local GGUF model on `:8888` via llama.cpp (matches `DEFAULT_HOST`) |
| `just check` | Cargo check (entire workspace) |
| `just check-engine` | Cargo check (`novelcraft-engine` only) |
| `just check-gui` | Cargo check (`novelcraft-gui` only) |
| `just clippy` | Cargo clippy (entire workspace) |
| `just fmt` / `just fmt-check` | **Do not use** — rustfmt is unsuited for this codebase's rsx-heavy UI code; formatting is manual |

## File Naming Conventions

### Rust Source Files
- snake_case: `main.rs`, `config.rs`, `error.rs`, `paths.rs`, `session.rs`, `turns.rs`, `state.rs`, `profile.rs`
- One file per gameplay module in `engine/src/game/module/`, named after the module struct (`story.rs` → `StoryModule`)
- Directories group by layer: `util/`, `markdown/`, `game/module/`

## Related Documentation

- [Code Conventions](./code-conventions.md) - Styling guidelines and import patterns
- [Data Storage](./database-schema.md) - JSON file formats and storage layout
- [Engine API](./api-routes.md) - Engine API reference
- [GUI Architecture](./gui-architecture.md) - Dioxus screens, components, and styling conventions
