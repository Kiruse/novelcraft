# GUI Architecture

This document describes the Dioxus-based GUI crate (`gui/`, binary `novelcraft-gui`).

## Overview

The GUI is a **Dioxus 0.7** application rendered inside the platform's system WebView (via `wry` — WebKitGTK on Linux, WebView2 on Windows, WKWebView on macOS). Components are Rust functions written with the `rsx!` macro (React/JSX-style), styled with Tailwind CSS utility classes, and reactivity is signal-based. Mobile (Android/iOS) is an officially supported compile target of the same codebase; desktop is the primary target today.

Text input (including IME) is handled natively by the WebView — the GUI ships no custom text input code.

### Key Technologies

- **dioxus** 0.7 (features: `desktop`, `router`) — UI framework: components, signals, router
- **Tailwind CSS v4** — styling; compiled via `@tailwindcss/cli` (see [Styling](#styling))
- **novelcraft-engine** (path) — Business logic library: memory-managed sessions, game agent loop, config/profile persistence
- **reqwest** — REST client for the inspiration service
- **chrono** — Local-time formatting of session timestamps (Home screen session cards)
- **dx** (dioxus-cli) — dev server (`dx serve`), bundler (`dx bundle`), hot-reload

### Crate Layout

```
gui/
├── Cargo.toml               # novelcraft-gui crate (dioxus, engine, reqwest, …)
├── Dioxus.toml              # dx config (app name, bundle identifier)
├── package.json             # Tailwind toolchain (@tailwindcss/cli, tailwindcss) — bun
├── input.css                # Tailwind source: @theme tokens, base rules
├── assets/tailwind.css      # Compiled stylesheet (build artifact, do not edit)
└── src/
    ├── main.rs              # Entry point — engine init, BOOT_CONFIG, launch, App
    ├── engine.rs            # Engine bridge — all engine access & query refreshes
    ├── state.rs             # Global reactive state — Query<T>, GlobalSignals
    ├── toast.rs             # Toast notifications (global signal + overlay component)
    ├── comps.rs             # Shared components (Screen, Button, Chip, Field, …)
    ├── titlebar.rs          # Window chrome — custom title bar + resize handles
    ├── api.rs               # Inspiration REST client (get_json)
    ├── error.rs             # GuiError (thiserror): Engine/Api variants
    └── screens/             # One module per screen + Route enum (mod.rs)
        ├── mod.rs           # Route enum (Routable) + go_back()
        ├── home.rs          # Session list
        ├── settings.rs      # Engine config editor (models, system prompt, summary threshold)
        ├── create_story.rs  # New vignette + inspiration browser
        ├── story_overview.rs# Session detail + Play/Delete (with confirm dialog) buttons
        └── story_gameplay.rs# Game view — turns, streaming, chat bar, memory sidebar
```

## Architecture

### Routing

Navigation is handled by the Dioxus Router. The `Route` enum (`gui/src/screens/mod.rs`) defines the URL space; each variant maps 1:1 to a screen component:

```rust
#[derive(Clone, Routable, Debug, PartialEq)]
pub enum Route {
  #[route("/")]
  Home {},
  #[route("/settings")]
  Settings {},
  #[route("/create")]
  CreateStory {},
  #[route("/story/:id")]
  StoryOverview { id: String },
  #[route("/play/:id")]
  StoryGameplay { id: String },
}
```

- Navigate with `navigator().push(Route::…)` — never `window.dispatch_action` (that was gpui; no action system exists anymore).
- "Back" is `go_back(from: Route)` (`screens/mod.rs`): gameplay returns to its story's overview, everything else to home. It is invoked by the shared `CloseButton` component, which reads the current route with `use_route::<Route>()`.
- Screens are remounted on every navigation, which replaces the old `enter()`/`exit()` lifecycle — per-screen state lives in `use_signal` and resets naturally. Mount-time side effects use `use_hook(|| …)` + `spawn`.

### Global State — Signals

All cross-screen state lives in `GlobalSignal`s (`gui/src/state.rs`). Each engine-derived value is wrapped in `Query<T>`, mirroring the semantics of the previous gpui `EngineQuery`:

```rust
#[derive(Debug, Clone, PartialEq, Default)]
pub enum Query<T> {
  #[default]
  Empty,
  Error(String),
  Stale(T, String), // previous value retained alongside a refresh error
  Recent(T),
}
```

| Signal | Type | Purpose |
|--------|------|---------|
| `CONFIG` | `Query<NovelCraftConfig>` | Engine config (seeds from `BOOT_CONFIG`) |
| `PROFILES` | `Query<Profiles>` | Profiles snapshot + active profile ID |
| `SESSIONS` | `Query<Vec<SessionV2>>` | Session list (metadata only) |
| `SESSION` | `Query<SessionV2>` | Active session |
| `TURN_INDEX` / `TURN` | `usize` / `Query<TurnV2>` | Current gameplay turn (window-relative index into the retained history) |
| `STREAM` | `Option<StreamState>` | In-flight prompt generation (prompt, text, activity) |

Conventions:

- **Read** with `X.read()` inside components (subscribes to changes). Clone the value out (`X.read().clone()`) when handing data into `rsx!` loops that capture it in event closures — event handlers must be `'static`.
- **Write** with `*X.write() = …` for globals (there is no `.set()` on statics) and `x.set(…)` / `x.write()` for local `use_signal`s, which need `let mut` bindings.
- **Stale-guard**: refreshes that can race (sessions, turn loads) use per-query `AtomicUsize` generation counters (`SESSIONS_GEN`, `TURN_GEN` in `gui/src/engine.rs`) — only the newest invocation publishes its result.

### Engine Bridge

`gui/src/engine.rs` is the single seam between UI and engine. The engine lives in a process-wide static and is shared behind a tokio mutex — access is serialized, replacing the old engine-thread + `CommandBus` + mpsc layer:

```rust
static ENGINE: OnceLock<Arc<Mutex<NovelCraftEngine>>> = OnceLock::new();
```

`main()` constructs the engine before launch (`NovelCraftEngine::new(config)`); the config is also stashed in `state::BOOT_CONFIG` so the `CONFIG`/`PROFILES` signals can seed themselves lazily on first read **inside** the Dioxus runtime (touching signals from `main()` panics — never do it).

Engine operations are `pub async fn`s that lock the engine, call it, publish results to the global signals, and toast on failure:

```rust
// fire-and-forget refreshes (spawn internally, generation-guarded)
engine::refresh_sessions();

// awaited operations, called from spawned tasks in components
engine::play_session(id).await;   // loads the session + opens its last retained turn
let session = engine::create_session(title, exposition).await?;
engine::switch_profile(id).await;
engine::save_config(config).await;

// turn navigation — indices are relative to the session's retained history
engine::load_turn(index); engine::goto(index);
engine::navigate(delta); engine::jump_to_end();

// long-term memory read model (rolling summary, chapters)
let memory = engine::memory_async().await?;
```

### Streaming Prompts

`engine::prompt(content, from_turn)` drives the agent loop:

1. Sets `STREAM` to a fresh `StreamState` (UI swaps the turn card for the stream card).
2. Spawns a worker task that locks the engine, optionally forks first (`from_turn = Some(index)` truncates the history at that turn — the gameplay screen does this whenever the user sends from any turn other than the last), and calls `NovelCraftEngine::prompt(content, tx)` with an mpsc sender.
3. A sibling task drains the chunk channel, absorbing `AgentMessageChunk`s into `STREAM` (`Content` appends text; `ToolCall*` sets the activity flag).
4. When the worker finishes (sender dropped → drain loop ends), `STREAM` is cleared, the session query is refreshed, and the current turn is reloaded — a fork may have truncated turns, and the current turn gained the fresh response either way (the reading position does not auto-advance; "Jump to end" navigates to the last retained turn).

While `STREAM` is `Some`, turn navigation and sending are blocked and the status row hides its indicators.

All turn indices (`load_turn`/`goto`/`navigate`/`jump_to_end`, `TURN_INDEX`) are **window-relative**: they index into the session's *retained* history (the last `2 × summary_threshold` turns), not the absolute conversation — older turns are compressed away into the rolling story summary by the engine (see `SessionV2::compress_memory`).

### Window Chrome

The window is launched with `.with_decorations(false)` (`main.rs`) — the platform-default title bar and any WM-drawn chrome are hidden, and `gui/src/titlebar.rs` provides themed replacements:

- **`TitleBar`** — a slim (h-8) top strip: the whole bar is a drag handle (`DesktopContext::drag()` via `onmousedown`), double-click toggles maximize, and it hosts minimize / maximize / close buttons wired to `set_minimized(true)`, `toggle_maximized()`, and `close()`. The buttons stop propagation so they don't start a drag.
- **`ResizeHandles`** — eight invisible fixed-position strips (4px edges, 12px corners, resize cursors) mapped to `tao::window::Window::drag_resize_window(ResizeDirection)`, restoring the edge-resize behavior the window manager suspends for undecorated windows.

Both are mounted in `App` above the router content. `DesktopContext` (from `dioxus::desktop::use_window()`) is a cheap Rc handle — clone one per event closure (`move` closures capture by move).

### Toasts

`gui/src/toast.rs` — toasts are a `GlobalSignal<Vec<Toast>>` + a fixed bottom-right `Toasts` component mounted in `App`. Any async task can raise one:

```rust
push_toast(Toast::error("Session creation failed"));
```

Each toast schedules its own dismissal task (default 5s). Toast variants (Info/Success/Warn/Error) map to Tailwind border accent classes.

### Shared Components

`gui/src/comps.rs` — stateless rsx components, replacing the old builder-function `comp.rs`:

| Component | Purpose |
|-----------|---------|
| `Screen` | Standard layout: centered title bar (gear → settings, or × when `closable`) + a full-width scrollable area (`flex-1 min-h-0 w-full overflow-y-auto`) containing a centered max-w-160 content column — the scrollbar sits at the window edge, not at the column |
| `Button` | Full-width button; variants `Primary`/`Secondary`/`DangerPrimary`/`DangerSecondary`, `disabled` flag, `on_click` handler |
| `Chip` | Rounded tag pill with `selected` toggle styling |
| `Field` / `FieldGroup` | Labeled input row / bordered group with heading |
| `Loading` | Pulsing text (`animate-pulse`) |
| `SettingsGear` / `CloseButton` | Top-bar icon buttons |

`INPUT_CLASS` is the shared text-input styling constant.

## Styling

Styling is **Tailwind CSS v4** with utility classes written directly in `rsx!` (`class: "flex flex-col gap-2 …"`).

- **Source of truth:** `gui/input.css` — `@import "tailwindcss"`, an `@source` directive scanning `src/**/*.rs` for class names, and the theme tokens under `@theme`:

```css
@theme {
  --color-nc-bg: #202833;
  --color-nc-text: #b1c5db;
  --color-nc-label: #E87813;
  --color-nc-border: #66666640;
  --color-nc-danger: #642C2C;
  --color-nc-dangerfg: #E88C8C;
  --color-nc-success: #5FA867;
  --color-nc-warn: #E8B813;
}
```

  These generate `bg-nc-bg`, `text-nc-label`, `border-nc-border`, … utilities. The old `theme.rs` / `Rgba` struct is gone; colors live entirely in CSS.

- **Build:** the compiled stylesheet is `gui/assets/tailwind.css` (a build artifact — do not edit). Compile it once with `just css`, or run the watcher alongside `dx serve` via `just dev`. The stylesheet is linked in `App` via `document::Stylesheet { href: asset!("/assets/tailwind.css") }`.
- **Toolchain:** `@tailwindcss/cli` + `tailwindcss` npm packages are provisioned with `bun` (see `gui/package.json`, `bun.lock`). `dx` (0.7.x) does **not** compile Tailwind itself — the watcher must be running.

## Dev Workflow

```bash
just dev    # Tailwind watcher + dx serve --platform desktop (hot-reload)
just css    # compile the stylesheet once (minified)
just build  # css + dx bundle --platform desktop (installers under target/dx)
just check  # cargo check (entire workspace)
```

Notes:

- `dx` must run from `gui/` (where `Dioxus.toml` lives); the justfile handles this.
- Linux desktop dev requires `webkit2gtk-4.1` and `libxdo` (`dx doctor` verifies).
- Plain `cargo run` also works for a quick boot, but the stylesheet won't recompile on edits (`just dev` is the supported loop).
- Mobile: `dx serve --android` / `--ios` require the NDK / Xcode toolchains (see the Dioxus mobile guide).

## Conventions

- Screens use the shared `Screen` wrapper; gameplay uses a custom root layout (top bar with ☰ sidebar toggle, turn viewport + optional sidebar, status row, chat bar).
- The gameplay sidebar (280px, toggled by the ☰ button) renders the session's **story memory** — the premise (from the session's exposition), rolling summary ("Story so far"), and chapters with their bullet notes — fetched via `engine::memory_async()` each time it opens (the `Sidebar` component is only mounted while open).
- Event handlers must not return values — end `nav.push(…)` and friends with `;` (Dioxus interprets non-unit returns as async spawns and fails to compile).
- Never touch global signals from `main()` or from plain helper functions outside components/tasks — signal access requires the Dioxus runtime.
- Engine access goes exclusively through `engine.rs`; screens never import `NovelCraftEngine` directly.
- Keyboard: the gameplay screen handles `alt+arrow-left/right` for turn navigation on its root `onkeydown` (with `prevent_default`, so the webview doesn't treat it as history navigation). Enter submits the chat bar; shift+enter inserts a newline.
- Swipes: the turn viewport tracks pointer down/move/up — a ≥60px horizontal drag flips turns.
