# GUI Architecture

This document describes the gpui-based GUI crate (`gui/`, binary `novelcraft-gui`).

## Overview

The GUI is a native Rust binary using the **gpui** framework (from the Zed editor repo) for rendering. There is no HTML, no CSS, no JavaScript. Screens are gpui view structs constructed via a `create(cx)` constructor and rendered through the `Render` trait. Navigation between screens is handled by an enum-based state machine in the root view.

### Key Technologies

- **gpui** (git, Zed main) — UI framework: views, elements, Tailwind-like styling via the `Styled` trait
- **gpui_platform** (git, Zed main) — Platform integration: window management, app lifecycle
- **novelcraft-engine** (path) — Business logic library for data persistence, LLM proxy, game loop
- **chrono** — Local-time formatting of session timestamps (used by the Home screen's session cards)
- **unicode-segmentation** — Grapheme-boundary cursor movement (used by the `TextInput` component)

## Architecture

### Screen-Based Navigation

The `Screen` enum (`gui/src/screens/mod.rs`) drives navigation. `AppRoot` matches on the current `Screen` variant and attaches the corresponding screen entity as a child.

```rust
#[derive(Debug, Clone, Default)]
pub enum Screen {
  #[default]
  Home,
  Settings,
  CreateStory,
  StoryOverview(StoryId),
  StoryGameplay(StoryId),
}
```

Each screen is a gpui view struct with a `create(cx)` constructor and a `Render` impl. Screens with no local state are unit structs; screens that need state hold it as fields (e.g. `StoryOverviewScreen.id: StoryId`).

### Screens

| Screen | Struct | State |
|--------|--------|-------|
| Home | `HomeScreen` | `task: Task<()>` — owns the `EngineQuery<Vec<SessionV1>>` watcher (see [Home Screen](#home-screen-homescreen)) |
| Settings | `SettingsScreen` | `config: Option<NovelCraftConfig>` + `task: Task<()>` (config watcher) + `Entity<TextInput>` per editable field (see [Settings Screen](#settings-screen-settingsscreen)) |
| Create Story | `CreateStoryScreen` | `title` / `premise`: `Entity<TextInput>`; Get Inspired: `categories` / `inspirations`: `Option<Result<_, GuiError>>`, `selected: BTreeSet<String>` + `inspirations_gen` generation guard (see [Create Story Screen](#create-story-screen-createstoryscreen)) |
| Story Overview | `StoryOverviewScreen` | `id: StoryId` |
| Story Gameplay | `StoryGameplayScreen` | `id: StoryId`; `watch_task: Task<()>` — owns the single query watcher, `stream: Option<Stream>`, `input: Entity<TextInput>`, `popover_open`, `sidebar_open`, swipe accumulators `swipe_x` / `swipe_last` — no duplicated domain state, all data is read from the global query caches (see [Story Gameplay Screen](#story-gameplay-screen-storygameplayscreen)) |

Each screen lives in its own submodule under `gui/src/screens/` (`home.rs`, `settings.rs`, `create_story.rs`, `story_overview.rs`, `story_gameplay.rs`); `mod.rs` declares the submodules, defines the `Screen` enum, and re-exports the screen structs. `StoryOverviewScreen` is currently a placeholder (`render` returns an empty `div()`).

Screens share a common layout through `ScreenBase` (`gui/src/screens/mod.rs`). A screen's `render` starts with the `screen(title)` helper, attaches content via `.child(...)` (`ScreenBase` implements `ParentElement`), and returns it directly (`ScreenBase` implements `IntoElement`). By default the top bar shows the settings gear; call `.closable()` to show the close (`Back`) button instead. The helper composes the `comp.rs` builders (`screen_root`, `top_bar`, `title`, `content`) so individual screens don't repeat that boilerplate:

```rust
screen(text!("NovelCraft"))            // gear in the top bar
  .child(create_vignette(theme))
  .child(subtitle(text!("Sessions")))
  .child(sessions_ui)

screen(text!("Settings")).closable()   // close button instead of gear
  .child(field(&theme, "Max Agent Steps", &self.max_agent_steps))
```

### AppRoot View

`AppRoot` (`gui/src/main.rs`) is the sole top-level gpui `View`. It holds:

- `screen: Screen` — current screen variant
- `screen_home` / `screen_settings` / `screen_create_story` / `screen_story_overview` / `screen_story_gameplay` — `Entity<...Screen>` for each screen, created once via `cx.new(|cx| ...Screen::create(cx))`
- `toasts: Vec<(usize, Toast)>` + `next_toast_id` — active toast queue (see [Toasts](#toasts))

The `Render` impl matches on `self.screen` and attaches the matching screen entity as a child. For `StoryOverview`/`StoryGameplay` it first syncs the `StoryId` into the screen entity via `cx.update_entity`. Navigation is performed by mutating `self.screen` (global action listeners do this via `root.update(cx, ...)`); gpui re-renders automatically since the entities are observed.

Engine communication uses two `mpsc` channels. GUI → engine: a `CommandBus(mpsc::Sender<Command>)` (both `pub(crate)`) is registered as a gpui `Global`; a dedicated engine thread runs a tokio runtime and consumes the receiving end. Engine → GUI: an `AppEvents` channel (`Toast(Toast)`, `UpdateConfig`, `SwitchSession`, `SwitchProfile`) — the engine thread reports command failures as toasts and signals state changes, and a task spawned in `main()` consumes the events, dispatching toasts and refreshing the matching global `EngineQuery` (`UpdateConfig` → config query, `SwitchProfile` → profiles query, `SwitchSession` → active-session query). `Command` is a `pub(crate)` enum with six variants:

| Variant | Payload | Semantics |
|---------|---------|-----------|
| `SwitchProfile(String)` | profile ID | Fire-and-forget: engine thread sets the global active profile via `engine.set_active_profile`, emits `AppEvents::SwitchProfile`, and persists the engine config; on persist failure it logs the error and toasts "Failed to save active profile" |
| `SaveConfig(NovelCraftConfig)` | config | Fire-and-forget: engine thread persists with `config.save().await`, syncs it into the engine via `engine.set_config`, and emits `AppEvents::UpdateConfig`; on failure it toasts "Failed to save new config" |
| `CreateSession { title, exposition }` | title, exposition | Fire-and-forget: engine thread calls `NovelCraftEngine::create_session(title, exposition)` — builds a fresh `SessionV1` with default gameplay modules and the engine's active profile, persists it — sets it as the active session via `set_session`, and emits `AppEvents::SwitchSession`; on error it clears the active session, logs the error, and toasts "Session creation failed" |
| `PlaySession { id }` | session ID | Fire-and-forget: engine thread calls `SessionV1::load(id, &config.profiles)` (full load incl. page counts + gamestate replay), makes it the engine's active session via `set_session`, and emits `AppEvents::SwitchSession`; on error it clears the active session, logs the error, and toasts "Failed to load session" |
| `GamePrompt { content, from_page, chunks, done }` | prompt, optional fork point, chunk channel, reply channel | Streaming request: engine thread forks at `from_page` via `engine.fork(..)` when set (deleting all pages after it), then runs `engine.prompt(content, chunks)` — agent chunks stream over the `mpsc` channel — and replies `Result<(), EngineError>` over the oneshot channel |
| `QueryEngine(querier)` | boxed higher-ranked closure | General-purpose escape hatch (primarily used internally by `EngineQuery`): the engine thread invokes `querier(&engine)` with shared access to the active `NovelCraftEngine` and awaits the returned future on the engine thread. The closure is typed `Box<dyn for<'a> FnOnce(&'a NovelCraftEngine) -> LocalBoxFuture<'a, ()> + Send>` — the future borrows the engine, so it must be a `LocalBoxFuture` tied to that borrow (not a `'static`/`Send` `BoxFuture`) |

`Command` derives nothing — the oneshot sender field of `GamePrompt` is neither `Debug` nor `Clone`. Commands are sent via `CommandBus::send` (a `blocking_send` whose error is logged through the `Loggable` trait) or the `Command::dispatch(cx)` convenience helper. `CreateSession`/`PlaySession` are fire-and-forget — their outcome is observed by refreshing `EngineQuery<SessionV1>` (see below).

`EngineQuery<T>` (also in `main.rs`) is the GUI's reusable engine-read primitive; instances are registered as gpui `Global`s keyed by type. `register_queries` installs five: `EngineQuery<NovelCraftConfig>` and `EngineQuery<Profiles>` (both seeded via `init` with the config loaded at startup — `Profiles` is a `main.rs` snapshot type `{ profiles, active_profile }` with `iter()`/`active_profile()` accessors), `EngineQuery<Vec<SessionV1>>` (querier: `NovelCraftEngine::list_sessions()`), `EngineQuery<SessionV1>` (querier: `engine.session()`), and the `PageQuery` wrapper described below.

All synchronization lives in a single `tokio::sync::watch` channel of `EngineQueryResult<T>`, plus an `Arc<AtomicUsize>` round counter and the querier — a dyn `EngineQuerier<T>` backed by an `async` closure `for<'a> AsyncFn(&'a NovelCraftEngine) -> Result<T, GuiError>` (boxed at the abstraction boundary via the `QuerierFn` adapter, since `AsyncFn` itself is not dyn compatible; stored behind `Arc`, hence `Send + Sync`). `EngineQueryResult<T>` is a four-state enum:

| Variant | Meaning |
|---------|---------|
| `Empty { round }` | Initial/reset state — the channel's default, (re)published by `clear()` |
| `Error { error, round }` | Refresh failed, no previous value to keep |
| `Stale { value, error, round }` | Refresh failed, but a previous value exists and is kept alongside the error |
| `Recent { value, round }` | Fresh success |

Accessors on the result: `round()`, `value() -> Option<&T>`, `error() -> Option<&GuiError>`, `is_empty()`, `is_stale()`. Accessors on the query: `rx()` returns a single cloned receiver marked unchanged — so `changed().await` resolves on the next publish, making "spawn a watcher, then refresh" the standard consumption pattern; `curr()` borrows the current result as a `watch::Ref`; `is_in_flight()` compares the round counter against the published round; `is_stale()` (true for `Empty` **or** `Stale` — no fresh value to show) and `has_error()` classify the current state. `refresh_with_bus(&CommandBus)` (or `refresh(&App)`, which extracts the global bus) dispatches a `QueryEngine` command — running the querier on the engine thread — and returns the receiver. The round counter guards superseding: each invocation claims a round and only the newest publishes (earlier results are dropped, and a `clear()` invalidates in-flight refreshes the same way). On success the result becomes `Recent`; on failure the previous value — when present — is downgraded to `Stale` with the new error, otherwise `Error` is published. `init(value)` seeds a `Recent` value; `clear()` resets to `Empty`. Callers write queriers as `async |engine: &NovelCraftEngine| { ... }` closures; the async block borrows the engine and is awaited on the engine thread, so no `Send`/`'static` bound applies to the future itself.

`PageQuery` (also in `main.rs`, also a global) wraps an `EngineQuery<PageV1>` with an `Arc<AtomicUsize>` page index: `load(page_index, &CommandBus)` stores the index (read on the engine thread, so a superseded in-flight load may query the newest index — its result is discarded by the round guard regardless), refreshes, and returns the single receiver; `page_index()` reads the stored index and `curr()` borrows the inner query's current `EngineQueryResult<PageV1>`; `clear()` resets, rewinds the page index to 0 (a newly entered session starts at page 0), and discards in-flight loads.

### Toasts

Toasts provide non-blocking input feedback and live entirely in `AppRoot` (`gui/src/main.rs`).

- **Data**: `Toast { variant: ToastVariant, msg, title: Option<String>, duration }` with constructors `Toast::info/success/warn/error(msg)` (5s default duration; `title: None` falls back to a variant default: Info/Success/Caution/Error). `ToastVariant::color(&theme)` maps Info → `theme.text`, Success → `theme.success`, Warn → `theme.warn`, Error → `theme.danger_fg`.
- **Spawning**: dispatch the `toast::SpawnToast(Toast)` action (`actions` module in `main.rs`, carries the full `Toast`). A global `on_action` listener calls `AppRoot::spawn_toast`, which assigns a sequential id, appends to `self.toasts`, and — when `duration != Duration::ZERO` — detaches a `cx.spawn`ed task that awaits `cx.background_executor().timer(duration)` and then dismisses the toast. `duration == 0` toasts stay until dismissed manually.
- **Removal**: `AppRoot::dismiss_toast(id)` retains all other toasts; both paths call `cx.notify()`.
- **Rendering**: when non-empty, `AppRoot::render` appends an absolute overlay (`bottom_4`, full width, centered column, `gap_2`) after the screen child — painted last, so it sits above screen content. Each toast (`toast_view` helper) is a bordered rounded box with a drop shadow (`shadow_lg`) (Error uses `danger_bg` bg + `danger_fg` border; others `theme.bg`/`theme.border`) with the variant-colored title, the message, and a "×" cancel button (top-right, styled like `btn_icon_close`) whose `on_click` listener dismisses that toast.
- **Ergonomics**: the `Toastable<T, E>` trait (`gui/src/util.rs`) is implemented for `Result` (mirroring `Loggable`): `report_toast(variant, cx)` dispatches a toast only on failure, and `info/success/warn/error_toast(cx)` are shortcuts that also pick the failure message from the error's `Display`. They take `cx: &mut App` — `Context` derefs to `App`, so view code passes its context directly; async code wraps with `AsyncApp::update`. Dispatching goes through `App::dispatch_action(&SpawnToast(...))`.

Toasts originate from two places: the engine thread sends `AppEvents::Toast` over the event channel for command failures (session creation/loading, config save, profile persistence — see the command table under [Architecture](#architecture)), which the GUI event loop dispatches; and GUI-side code dispatches its own via `Toastable` (Get Inspired fetches, gameplay prompt submission) or directly (the Settings save button's success toast).

## Reusable Components (`gui/src/comp.rs`)

`comp.rs` contains **stateless builder functions** — plain functions returning styled elements. They hold no state and register no key bindings.

| Function | Returns | Purpose |
|----------|---------|---------|
| `root(theme: &Theme)` | `Div` | Root of most screens: flex column, items centered, theme bg + text color |
| `screen_root()` | `Div` | Screen content wrapper: flex column, items centered, `w_full h_full` |
| `content()` | `Stateful<Div>` | Scrollable content column: `w_full`, `max_w(px(640.))`, `gap_4`, `p_4` |
| `top_bar()` | `Div` | Title bar row: `relative w_full`, flex row, `justify_center` |
| `title(content: Text)` | `Div` | Screen title text at `text_3xl()` |
| `subtitle(content: Text)` | `Div` | Section subtitle text at `text_2xl()` |
| `field(theme, label, input)` | `Div` | Labeled form field: label above an `Entity<TextInput>` |
| `field_group(theme, label)` | `Div` | Bordered group box with a heading (used for model groups) |
| `create_text_input(cx, multiline, placeholder)` | `Entity<TextInput>` | Creates a `TextInput` entity with the given mode and placeholder |
| `button(anim_id, label)` | `IncompleteButton` | Starts a themed `Button` (see below) |
| `chip(anim_id, label)` | `IncompleteChip` | Starts a themed toggle `Chip` (see below) |
| `loading_text(anim_id, text)` | `AnimationElement<Div>` | Pulsating loading placeholder: the text breathes between 0.2–1.0 opacity on a 1s repeating gpui animation |
| `settings_gear()` | `impl IntoElement` | Gear icon button (see below) |
| `btn_icon_close()` | `impl IntoElement` | Close icon button (see below) |

### `button()` — Themed Buttons

`button(anim_id, label)` returns an `IncompleteButton` whose variant methods apply the `Theme` and produce a `Button` (an `IntoElement` wrapper around a styled, full-width `Stateful<Div>`). The four variants:

| Variant | Background | Text | Border |
|---------|-----------|------|--------|
| `primary(&theme)` | theme `text` | theme `bg` | none (transparent) |
| `secondary(&theme)` | transparent | theme `text` | theme `text` |
| `danger_primary(&theme)` | `danger_bg` | theme `bg` | none (transparent) |
| `danger_secondary(&theme)` | transparent | `danger_fg` | `danger_fg` |

Hover states are color-only (no opacity): `primary` dims its background (`text` @ 85% alpha), `danger_primary` lightens `danger_bg` lightness by `+0.15` (`HslaExt::lum`), and the outlined variants tint the background with their foreground @ 10% alpha (`danger_secondary` also lightens its text/border). Pressed states (mouse down until release, via gpui's `active` style — triggered by any mouse button) darken `primary`/`danger_primary` by `-0.1` lightness and strengthen the outlined variants' background tint to 20% alpha; pressed colors override hover. The 1px border is always present in the element so layout is stable; "no border" variants use a fully transparent border color.

Since `Button` only implements `IntoElement` (not interactivity), attach `on_click` by converting first:

```rust
button("btn-save", text!("Save"))
  .primary(&theme)
  .into_element()                // -> Stateful<Div>
  .on_click(cx.listener(|this, _, _, cx| this.save(cx)))
```

### `chip()` — Toggle Chips

`chip(anim_id, label)` mirrors `button()`: it returns an `IncompleteChip` whose variant method applies the `Theme` and produces a `Chip` (an `IntoElement` wrapper around a styled `Stateful<Div>`). The single variant, `select(&theme, selected)`, styles one of two toggle states:

| State | Background | Text | Border |
|-------|-----------|------|--------|
| `select(&theme, true)` | theme `text` | theme `bg` | none (transparent) |
| `select(&theme, false)` | transparent | theme `text` | theme `border` |

Hover follows the button conventions: the toggled-on chip dims its background (`text` @ 85% alpha, like `primary`); the toggled-off chip tints its background with `theme.text` @ 10% alpha and raises its border to `theme.label`. Unlike `Button` there are no pressed or disabled states. The `Chip` renders a content-sized pill — `px_2 py_0p5 text_sm rounded_full border_1` with `cursor_pointer`; as with `Button`, the 1px border is always present in the element and "no border" means a transparent border color. No click handler is built in — convert first, then attach `on_click` yourself:

```rust
chip(format!("chip-category-{id}"), text!(label))
  .select(&theme, self.selected.contains(&id))
  .into_element()                // -> Stateful<Div>
  .on_click(cx.listener(move |this, _, _, cx| this.toggle_category(&id, cx)))
```

First consumer: the Create Story screen's category filter row (see [Get Inspired](#get-inspired-local-mock-data)).

### `settings_gear()` and `btn_icon_close()`

Both are icon buttons with the same interaction pattern:

- `settings_gear()` renders `"⚙"` (`\u{2699}`), `btn_icon_close()` renders `"×"` (`\u{00d7}`), both at `text_xl()`
- `.id(...)` for interactivity, `.absolute().right_4()` positioning inside a `top_bar()`
- Pointer cursor on hover, opacity drops to 0.7 on hover
- `on_click` dispatches an action via `window.dispatch_action(...)` — `ShowSettings` for the gear, `Back` for the close button. The global action listeners in `main()` perform the navigation.

This is the "stateless component + action dispatch" pattern: the component does not know about screens; navigation is decided by the app-level action handlers.

## Component Module Convention

- **Stateless builders** — free functions in `gui/src/comp.rs` (see above).
- **Stateful interactive components** — standalone modules `gui/src/<component>.rs` with their own struct, `create(cx)` constructor, `Render` impl, and (when needed) a module-level `init(cx)` that registers key bindings. `gui/src/text_input.rs` is the current example: it defines a custom `Element`, a scoped key context, and an `EntityInputHandler`.

`init(cx)` functions are called once from `application().run(...)` in `main.rs`, before any window is opened.

## TextInput Component (`gui/src/text_input.rs`)

A reusable text input component (adapted from Zed's official gpui input example). One struct serves both single-line and multiline modes via a `multiline: bool` field. `SettingsScreen` is its first consumer. The file still carries `#![allow(dead_code)]` because parts of its API surface (e.g. `on_submit`, `reset`) are not exercised yet.

### Public API

```rust
pub(crate) struct TextInput {
  pub multiline: bool,                 // single-line vs multiline mode
  pub placeholder: SharedString,       // shown at 35% opacity when content is empty
  pub on_submit: Option<SubmitCallback>,
  // ... private: focus_handle, content, selected_range, selection_reversed,
  //     marked_range, last_layout, last_multiline, last_bounds,
  //     last_wrap_width, is_selecting
}

pub(crate) type SubmitCallback = Rc<dyn Fn(&str, &mut Window, &mut App)>;

impl TextInput {
  pub(crate) fn create(multiline: bool, cx: &mut Context<Self>) -> Self;
  pub(crate) fn value(&self) -> &str;
  pub(crate) fn set_value(&mut self, value: impl Into<SharedString>, cx: &mut Context<Self>);
  pub(crate) fn reset(&mut self, cx: &mut Context<Self>);
}
```

`on_submit` receives the current content; the callback decides whether to clear the field afterwards (via `set_value`/`reset`) — the component never clears on its own.

**Usage:**

```rust
use std::rc::Rc;

let input = cx.new(|cx| {
  let mut input = TextInput::create(true, cx); // multiline
  input.placeholder = "Describe your story...".into();
  input.on_submit = Some(Rc::new(|value: &str, _window, _cx| {
    log::info!("submitted: {value}");
  }));
  input
});

// Render inside any element tree:
// .child(input.clone())

// Read / write from outside the entity:
let text = input.read(cx).value().to_string();
input.update(cx, |input, cx| input.reset(cx));
```

### Key Bindings

`text_input::init(cx)` is called once as the first statement inside `application().run(...)` in `main.rs`. It registers all bindings via `cx.bind_keys`, **every one scoped to the `"TextInput"` key context** — so they only fire while a `TextInput` is focused. Modifier combos that differ per platform (`ctrl-a/v/c/x`) are bound in both `ctrl-*` and `cmd-*` variants for portability.

| Keys | Action |
|------|--------|
| `backspace`, `delete` | Delete backwards / forwards (grapheme-aware) |
| `left`, `right` | Move cursor (grapheme-aware) |
| `up`, `down` | Move cursor by visual row (multiline only) |
| `shift-left/right/up/down` | Extend selection |
| `ctrl-a` / `cmd-a` | Select all |
| `home`, `end` | See behavior table below |
| `ctrl-v` / `cmd-v`, `ctrl-c` / `cmd-c`, `ctrl-x` / `cmd-x` | Paste / copy / cut |
| `enter` | Newline (multiline) or submit (single-line) |
| `ctrl-enter` | Submit (both modes) |
| `ctrl-cmd-space` | Show character palette |

The actions themselves are declared with `actions!(text_input, [Backspace, Delete, Left, Right, Up, Down, SelectLeft, SelectRight, SelectUp, SelectDown, SelectAll, Home, End, ShowCharacterPalette, Paste, Cut, Copy, Enter, Submit])`.

### Single-Line vs Multiline Behavior

| Behavior | `multiline: false` | `multiline: true` |
|----------|--------------------|-------------------|
| `Enter` | Invokes `on_submit` | Inserts `\n` |
| `Ctrl+Enter` (Submit) | Invokes `on_submit` | Invokes `on_submit` |
| `Up`/`Down` (+Shift) | No-op | Move/extend cursor by visual row |
| `Home`/`End` | Start / end of content | Start / end of current *source* line |
| Typed / IME / pasted newlines | Filtered to spaces | Preserved |
| Height | One `line_height` | Grows with content (visual rows × `line_height`) |
| Word wrap | No | At element width |
| Mouse selection | Yes | Yes |

Multiline mode currently has no scrolling or max-height — the field grows vertically with its content (the Settings screen's System Prompt field uses this mode).

### Rendering

The `Render` impl returns a container `div()` with:

- `.key_context("TextInput")` — scopes the key bindings above
- `.track_focus(&self.focus_handle(cx))` and a `Focusable` impl
- `.cursor(CursorStyle::IBeam)` over the whole field
- An `.on_action(cx.listener(Self::...))` handler per action — 19 total (bubble phase)
- Mouse listeners: `on_mouse_down`, `on_mouse_up`, `on_mouse_up_out`, `on_mouse_move` (click-to-position, drag selection in both modes)

The only child is an inner `div()` styled from the `Theme` global — `.p_2().bg(theme.bg).border_1().border_color(theme.text.opacity(0.25)).text_color(theme.text)` — containing the custom **`TextElement { input: cx.entity() }`**.

`TextElement` implements gpui's `Element` trait with the standard three phases:

- **`request_layout`** — width `relative(1.)`; height is one `line_height` in single-line mode, or (shaped visual rows × `line_height`) in multiline mode, which is what makes the field grow with content.
- **`prepaint`** — shapes the display text (content, or placeholder at 35% opacity when empty) into `TextRun`s via `ime_runs()` (which underlines the IME marked range), then:
  - *Single-line:* `window.text_system().shape_line(...)` → one `ShapedLine`; cursor/selection quads from `line.x_for_index`.
  - *Multiline:* `window.text_system().shape_text(..., Some(wrap_width))` → one `WrappedLine` per **source** line, stored as `MultilineLine { start, line }` with a running UTF-8 offset. The cursor is a single quad on the cursor's visual row; a selection produces one quad per **visual** (wrapped) row, computed via `wrap_row_ranges`.
- **`paint`** — registers the IME bridge with `window.handle_input(&focus_handle, ElementInputHandler::new(bounds, input), cx)`, paints selection quads → lines (`TextAlign::Left`) → cursor quad (only while focused), and writes the shaped lines plus `last_bounds`/`last_wrap_width` back onto the input for hit-testing and keyboard navigation.

Module-private helpers used by both the element and the input:

| Helper | Purpose |
|--------|---------|
| `line_entry_index_for_offset` | Map a content offset → `(source line ix, local offset)` |
| `rows_before` | Visual rows above a given source line |
| `wrap_row_ranges` | Byte ranges of each visual row within a `WrappedLine` |
| `row_for_local` | Visual row containing a local offset |
| `x_for_local_in_row` | X position of a local offset within a visual row |
| `ime_runs` | Build `TextRun`s, underlining the IME marked range |

### IME Support

`TextInput` implements `EntityInputHandler` (registered per-paint via `ElementInputHandler`), giving it full IME / candidate-window support:

- UTF-16 ↔ UTF-8 offset conversion for all range-based methods (`offset_from_utf16`, `offset_to_utf16`, …)
- The marked (composition) range is tracked and rendered underlined via `ime_runs()`
- **Single-line newline filtering is centralized**: `filtered_text()` replaces `\n`/`\r` with spaces and is applied inside both `replace_text_in_range` and `replace_and_mark_text_in_range`, so IME commits *and* clipboard paste are both covered
- `bounds_for_range` / `character_index_for_point` position the IME candidate window; multiline mode resolves through the per-visual-row math helpers

## Screen Details

### Home Screen (`HomeScreen`)

The Home screen is the app's session launcher. It uses the standard layout: `screen_root()` containing a `top_bar()` with the app title `"NovelCraft"` at `text_3xl()` (centered) and `settings_gear()` (absolute top-right, dispatches `ShowSettings`).

#### State & Load Flow

`HomeScreen` holds a single field, `task: Task<()>` — it keeps no session data itself; the data lives in the global `EngineQuery<Vec<SessionV1>>`. `create(cx)` calls `refresh(cx)` on that query (querier: `NovelCraftEngine::list_sessions()`) and spawns a watcher task on the returned receiver — a `while let Ok(()) = rx_sessions.changed().await` loop that calls `cx.notify()` on every publish, so the screen re-renders whenever the query state changes. The receiver is marked unchanged at creation, so the watcher fires exactly once for the refresh triggered right before it.

#### Render

Below the top bar, a `content()` column renders:

- **"Create new Vignette" card** — a jumbo full-width clickable bordered card (`create_vignette(theme)` helper): a horizontally+vertically centered title row (`flex().items_center().justify_center().gap_2()`) with a "+" text glyph before the "Create new Vignette" text, both at `text_2xl()`, plus the description that Vignettes are free-form stories that don't follow any template, resembling other story generator platforms. Hover swaps the border color to the label color; clicking dispatches the existing `nav::CreateStory` action via `window.dispatch_action`.
- **"Sessions" section** — a `subtitle(text!("Sessions"))` heading followed by the sessions list, rendered by the `sessions_ui` helper from the query's current `EngineQueryResult` (read via `curr()`). It matches on `(result.value(), result.error())` to respect all logical states:
  - `(None, None)` (loading) — a "Loading ..." placeholder pulsating via gpui's animation API: `with_animation(..., Animation::new(1s).repeat().with_easing(pulsating_between(0.2, 1.0)), |el, delta| el.opacity(delta))`
  - `(None, Some(_))` — the static text "Failed to load sessions" in `theme.danger_fg`
  - `(Some([]), _)` — the static text "No sessions yet"
  - `(Some(sessions), _)` — a column (gap-2) of clickable session cards

  When `result.is_stale()` (refresh failed but a previous list exists), a danger-colored note — "Failed to refresh — showing the last known sessions" — is rendered above the list, so stale data is never shown silently.

Each session card is built by the `session_card(theme, session)` helper: a bordered, hover-highlighted, clickable row showing the session title (`text_lg`), the `updated_at` timestamp formatted in local time via `chrono` (`%Y-%m-%d %H:%M`), and the exposition text. Cards get their session ID as the element ID; `on_click` dispatches `nav::ShowStory(StoryId)` where `StoryId` wraps the session ID. Both card types dispatch via `window.dispatch_action` and navigate through the app-level action listeners in `main()`.

### Create Story Screen (`CreateStoryScreen`)

The vignette creation form, reached from the Home screen's "Create new Vignette" card (`nav::CreateStory` → `Screen::CreateStory`). The top bar shows the title `"Create Vignette"` at `text_3xl()` (centered) with `btn_icon_close()` (`Back` → `Screen::Home`).

#### Fields

| Screen field | Type | Editor |
|--------------|------|--------|
| `title` | `Entity<TextInput>` | single-line, placeholder `"Title"` |
| `premise` | `Entity<TextInput>` | multiline, placeholder `"Describe the premise of your story ..."` |
| `categories` | `Option<Result<Vec<InspirationCategory>, GuiError>>` | — (chip filter row, populated by `load_categories()`) |
| `inspirations` | `Option<Result<Vec<Inspiration>, GuiError>>` | — (inspiration cards, populated by the debounced fetch) |
| `selected` | `BTreeSet<String>` | ids of the toggled-on category chips |
| `inspirations_gen` | `usize` | fetch generation counter (stale-response guard) |

Both inputs are created with the `create_text_input(cx, multiline, placeholder)` comp helper and rendered via `field(theme, label, input)`. The Create button is a `button("btn-create", text!("Create")).primary(&theme).disable(!valid)` — `valid` (both inputs non-empty when trimmed) is recomputed on every render via `cx.read_entity`, so the button renders muted with no hover/press states until the form is filled; otherwise it is identical to the Settings save button (see [`button()` — Themed Buttons](#button--themed-buttons)).

#### Submit Flow

`submit()` (bound to the Create button via `cx.listener`) reads both values trimmed and no-ops when the title is empty. Otherwise it dispatches `Command::CreateSession { title, exposition }` and **then** calls `EngineQuery::<SessionV1>::refresh(cx)` on the global query — commands execute in order on the engine thread, so the awaited refresh observes the freshly created (or failed) session. Inside a `cx.spawn`ed (detached) task it awaits `rx_session.changed()`: only when the result has no error **and** carries a value does it dispatch the `nav::ShowStory(StoryId(session.id))` action via `cx.update(|cx| cx.dispatch_action(...))` — allowed here because the spawned future runs outside a window update (note that `AsyncApp::update` returns the closure result directly, not a `Result`). The app-level listener routes the app to `Screen::StoryOverview`. Failures never navigate — they are toasted by the engine thread instead (see [Toasts](#toasts)).

#### Enter/Exit Lifecycle

`AppRoot::switch_screen` invokes `CreateStoryScreen::exit` when navigating away and `enter` when navigating back. `exit` resets both text inputs (`TextInput::reset`) and tears down the Get Inspired state: both lists return to `None`, `selected` is cleared, and `inspirations_gen` is bumped so any in-flight fetch is invalidated and cannot repopulate stale data into the hidden screen. `enter` re-kicks off `load_categories`/`fetch_inspirations` for anything still `None`, so every visit reloads the section.

#### Get Inspired (Local Mock Data)

Below the Create button, a `subtitle(text!("Get Inspired"))` section offers a category filter row plus a list of inspiration cards. **Everything is local to `create_story.rs`** — the engine is not involved. The data comes from module-private async mocks marked with a TODO to be replaced with REST calls once the inspiration service exists:

- `InspirationCategory { id, label }` and `Inspiration { title, premise, categories }` (`categories` holds category ids) — module-private types
- both lists are typed `Option<Result<Vec<_>, GuiError>>` — `None` means "still loading"; the fetched `Result` is kept as-is and only interpreted at render time
- `GuiError` (`gui/src/error.rs`, via `thiserror`) — the GUI-side error type: `Engine(EngineError)` (via `#[from]`), plus `Io`/`Api` string-carrying variants (with `io()`/`api()` constructors) reserved for IO and the future REST calls
- `fetch_categories()` — returns the mock category list
- `fetch_inspirations(selected: &[String])` — returns the mock inspirations OR-filtered by the selected category ids (an empty selection matches everything), mirroring where the future server-side filter will live

`create(cx)` kicks off `load_categories(cx)` and an initial, non-debounced `fetch_inspirations(cx)`; `enter(cx)` re-triggers them for anything still `None` (see the lifecycle above).

**Filter flow — debounce + generation guard:** `toggle_category(id)` (bound to each chip's `on_click`) flips the id in `selected`, bumps `inspirations_gen`, and spawns a task that awaits `cx.background_executor().timer(INSPIRATIONS_DEBOUNCE)` (300ms) and bails if the generation went stale, then fetches with the captured selection and populates only after a second generation check — rapid chip clicks coalesce into one fetch, and superseded responses are discarded. (The captured counter local is named `generation` because `gen` is a reserved keyword in Rust edition 2024.) On completion the raw `Result` is stored as `Some(...)` via `Toastable::error_toast(cx)`, which dispatches an error toast on failure (see [Toasts](#toasts)).

**Render:** the two lists render through state-matched helpers:

- `categories_ui(...)` — a wrapping chip row (`flex_wrap`, `gap_2`); each chip id is `chip-category-{id}` and its `on_click` routes to `toggle_category` via `cx.listener` (see [`chip()` — Toggle Chips](#chip--toggle-chips))
- `inspirations_ui(...)` — a column of static `inspiration_card(theme, inspiration, labels)` cards: bordered `rounded_sm` boxes with the title at `text_lg`, the premise, and small pill-shaped category labels (`theme.label` text, `theme.border` border) resolved through the `category_labels()` id→label map (unknown ids render raw). An empty result renders "No inspirations found"

Both helpers use a `let Some(res) = ... else` guard: while `None` they return `loading_text(anim_id, "Loading ...")` (the shared comp helper, see the component table); an `Err` renders plain theme-colored inline text ("Failed to query inspirations" / "Failed to load inspirations") — the details were already reported via toast by the fetch task.

### Story Gameplay Screen (`StoryGameplayScreen`)

The play view, reached from the (future) story overview via `nav::PlayStory(StoryId)`; `Back` returns to `Screen::StoryOverview` (see `AppRoot::on_back`). Unlike the other screens it does not use the `screen(title)` helper — it is a fixed three-region layout (page / chat bar / side bar), not a scrolling content column. The screen keeps only pure UI state: all domain data (session, page, profiles) is read from the global query caches (`EngineQuery<SessionV1>`, `PageQuery`, `EngineQuery<Profiles>`), so it stays in sync across the app.

#### Lifecycle

`create(cx)` spawns a single watcher task (`watch_task`) that `tokio::select!`s over three receivers — `PageQuery::rx()`, `EngineQuery<SessionV1>::rx()`, and `EngineQuery<Profiles>::rx()` — and calls `cx.notify` on the screen for any change, so a publish on any of the three queries re-renders the screen.

`AppRoot::switch_screen` calls `StoryGameplayScreen::enter(id, cx)` when the screen becomes visible. `enter` stores the requested `StoryId`, closes the popover, then — after `reset(cx)` — dispatches `Command::PlaySession` (the `EngineQuery<SessionV1>` querier reads the engine's ACTIVE session, so the command is required to switch sessions) and refreshes both `EngineQuery<SessionV1>` and `EngineQuery<Profiles>`. The session refresh is scheduled explicitly because the engine thread does not emit the `SwitchSession` event on failure — commands execute in order on the engine thread, so the awaited refresh observes the loaded (or failed) session. A detached task awaits the refresh publishing and opens the session's last page via `goto(count - 1)`.

`reset` clears the per-session UI state (`stream`, swipe accumulators) and calls `PageQuery::clear()` on the global — discarding the previous session's page and any in-flight load, and rewinding its page index to 0 so a newly entered session starts at page 0.

#### Layout & Page Navigation

The screen renders `root(&theme)` with: a custom top bar (sidebar toggle `\u{2630}` on the left, centered session title, `btn_icon_close()` on the right); a middle row with the page viewport (`flex_1`, `overflow_y_scroll`) plus the side bar when open; a status row (page indicator `k / n` and the "Jump to end" pill, only when not on the last page); and the chat bar. The top bar's title comes from the session query cache (`"..."` until a value is published).

Pages are fetched through the global `PageQuery`: `goto(index)` clamps to the cached session page count (`session_page_count(cx)` reads `EngineQuery<SessionV1>::curr().value()`) and calls `PageQuery::load(index, cx)`; `navigate(delta)` computes the clamped target from `PageQuery::page_index()`. Both are no-ops when the session cache holds no value or zero pages, and both are locked while a `stream` is active. There is no dedicated fetch loop — the single `watch_task` re-renders whenever the page query publishes, and superseded fetches are discarded entirely by the query's round guard (no screen-side generation counter). Three triggers:

- **Swipe** — the viewport starts a gpui drag with a unit `PageSwipe` payload and an invisible `DragGhost` view, and tracks `on_drag_move(DragMoveEvent<PageSwipe>)`: horizontal movement accumulates in `swipe_x` (anchored at `swipe_last`, reset on `on_mouse_down` and after each flip; per-move jumps larger than `4 × SWIPE_THRESHOLD` (240px) are treated as gesture-boundary artifacts and re-anchor instead). Crossing `SWIPE_THRESHOLD` (60px) — swipe left → next page, swipe right → previous — flips the page mid-drag.
- **Keyboard** — `alt-left` / `alt-right` fire the global `nav::PagePrev` / `nav::PageNext` actions; `AppRoot::gameplay_nav` forwards to `navigate` only while the gameplay screen is showing.
- **Jump to end** — the `status_row_ui` pill shown when `PageQuery::page_index() + 1 < session_page_count(cx)`.

#### Rendering a Page

`page_content_ui(theme, cx)` renders from the caches, in precedence order: streaming takes precedence (below); then the session query (error → "Failed to load session", no value → "Loading session ...", with the failure itself already toasted by the engine thread); then the fresh-session empty state (`page_count == 0` → the "Send your first prompt to begin the story ..." intro, without fetching any page); then `PageQuery` (error → "Failed to load page", no value → `loading_text`); an empty fetched page (`page_is_empty`) renders the same intro; and finally `page_card` — the page's prompt in a muted bordered box followed by each `AgentResponseV1::content` as plain story text (tool calls/reasons are part of the persisted conversation but are not rendered). The `status_row_ui` indicator reads the current index from `PageQuery::page_index()`.

#### Chat Bar & Profile Switching

The chat bar (bottom row) is: the profile avatar (a `rounded_full` circle with the active profile's initial, `"?"` when none), the multiline `TextInput` (`flex_1 min_w_0`, placeholder `"What do you do or say?"`; `Enter` = newline, `ctrl-enter` fires its `on_submit` hook which routes to `submit_input` like the Send button), and a compact Send button (ad-hoc `div`, primary colors when enabled — muted when `stream` is active, the input is empty, or no session is loaded).

The avatar toggles a `popover_open` popover (`absolute bottom_full left_0`, list of `ProfileV1` names, check mark on the active one, "No profiles yet" fallback). Both `avatar_ui` and `profile_popover_ui` read the profile list live via `EngineQuery<Profiles>::curr().value()`. The active profile is the **session's stored profile**: `active_profile_id(cx)` reads it from the session query cache, falling back to the global profiles query's `active_profile` value (the global active profile only matters for new sessions). Selecting a profile closes the popover and `switch_profile` dispatches `Command::SwitchProfile(id)`, then refreshes BOTH the Profiles and `SessionV1` queries — no optimistic local mutation, since the engine's `set_active_profile` also updates the active session's profile (see the Command table); the engine's own `SwitchProfile` event triggers another profiles-query refresh via the GUI event loop.

#### Prompt Submission & Fork Flow

`submit_input` reads/trim-checks/resets the input, then `submit(content)` (no-op when no session is loaded): when not on the last page, `from_page = Some(page_index)` — the engine thread forks away all pages after the viewed one before prompting; on the last page `from_page = None`. The screen builds a per-request `mpsc` channel for `AgentMessageChunk`s plus a oneshot reply channel (`Result<(), EngineError>`), sets `stream = Some(Stream { prompt, .. })`, dispatches `Command::GamePrompt`, and detaches a reader task that:

- appends `Content` chunks to `stream.text` and flags `stream.activity` on `ToolCallStart`/`ToolCallArgs` (rendered as a pulsating "The DM is taking action ..." line; `Reasoning`/`Done` chunks are ignored);
- ignores everything if the screen switched sessions meanwhile (the task captures the submit-time session id and bails on mismatch);
- after the chunk channel closes, awaits the reply: failures are toasted via `Toastable::report_toast(ToastVariant::Error, cx)`; then — regardless of outcome — `stream` is cleared, `EngineQuery<SessionV1>` is refreshed (a fork may have truncated pages), and the CURRENT page is reloaded directly via `PageQuery::load(current, cx)` — bypassing `goto`'s count guard, which also covers fresh sessions whose cached count is still 0.

While `stream` is set, the viewport renders `stream_card` (the prompt box + streamed text, with "Thinking ..." / "The DM is taking action ..." placeholders before text arrives) instead of the fetched page, and navigation/submit are locked.

#### Side Bar

`sidebar_open` (toggled from the top bar) shows a `SIDEBAR_WIDTH` (280px) right panel with a left border and a placeholder text — game state display/manipulation is deferred.

### Settings Screen (`SettingsScreen`)

A functional editor for the engine's `NovelCraftConfig` (persisted at `{configDir}/NovelCraft/config.json`). The top bar keeps the title `"Settings"` at `text_3xl()` (centered) and `btn_icon_close()` (absolute top-right); clicking `×` dispatches `Back`, which `AppRoot::on_back` maps to `Screen::Home` (or `Screen::StoryOverview` when returning from gameplay).

#### Fields

| Screen field | Type | Editor |
|--------------|------|--------|
| `config` | `Option<NovelCraftConfig>` | — (kept as the save base) |
| `max_agent_steps` | `Entity<TextInput>` | single-line, placeholder `"10"` |
| `system_prompt` | `Entity<TextInput>` | multiline, no placeholder |
| `models` | `ExhaustiveMap<ModelPurpose, ModelFields>` | one group per `ModelPurpose`, labeled via `purpose.as_str()` |

`ModelFields` holds one `Entity<TextInput>` per `ModelConfig::OpenAi` field — `base_url` (placeholder `DEFAULT_HOST`, i.e. `http://localhost:8888/v1`), `api_key`, and `model`. All inputs are built via the `create_text_input(cx, multiline, placeholder)` comp helper; `create(cx)` assembles the screen and immediately calls `load(cx)`.

#### Load Flow (Global Config Query)

```rust
fn load(&mut self, cx: &mut Context<'_, Self>) {
  let config = cx.global::<EngineQuery<NovelCraftConfig>>().curr().value().cloned();
  if let Some(config) = config {
    self.populate(config, cx);
  }

  let mut rx_config = {
    let q = cx.global::<EngineQuery<NovelCraftConfig>>();
    q.refresh(cx);
    q.rx()
  };

  self.task = cx.spawn(async move |this, cx| {
    while let Ok(()) = rx_config.changed().await {
      let config = rx_config.borrow().value().cloned().unwrap_or_default();
      this.update(cx, move |this, cx| this.populate(config, cx)).ok();
    }
  });
}
```

The config lives in the global `EngineQuery<NovelCraftConfig>` — seeded via `init` at startup and refreshed whenever the engine thread emits `UpdateConfig` (see the command table under [Architecture](#architecture)). `load` populates the inputs immediately from `curr()` when a value already exists, then triggers a `refresh(cx)` and spawns a watcher on the returned receiver: a `changed().await` loop that repopulates on every publish for as long as the screen entity lives (the watcher is stored in `task`). The future is detached.

`populate(config, cx)` fills every input via `TextInput::set_value` (`max_agent_steps` via `to_string()`, model fields destructured from the `ModelConfig::OpenAi` variants) and calls `cx.notify()`.

#### Save Flow

`save()` is bound to the Save button via `cx.listener`. It clones `self.config` (`unwrap_or_default()` if no value was ever populated) and rebuilds the editable parts from the inputs:

- `max_agent_steps` — input text trimmed and parsed; falls back to the previous value when unparsable (the input is not restricted to numeric text yet)
- `system_prompt` — raw input text
- Model groups — each `ModelPurpose` visited via `ModelPurpose::iter_all()`, its `ModelFields` mapped to `ModelConfig::OpenAi { base_url, api_key, model }` (base URL and model trimmed) via the `model_config()` helper

The result is dispatched as `Command::SaveConfig(config)`. The engine thread persists it with `config.save().await`, syncs it via `engine.set_config`, and emits `AppEvents::UpdateConfig` — refreshing the global query, which repopulates the inputs through the watcher; on failure it toasts "Failed to save new config". The button handler also dispatches `Toast::success("Settings saved!")` right after `save(cx)` — an optimistic confirmation (see [Toasts](#toasts)).

#### Layout

`screen(text!("Settings")).closable()` — the standard `ScreenBase` layout, whose `content()` column is the scrollable wrapper (`overflow_y_scroll`, `max_w(px(640.))`, `gap_4`, `p_4`). Fields are rendered by the `field(theme, label, input)` helper (label above the input). Each model entry is rendered by the module-private `model_group(theme, label, fields)` helper, wrapping `field_group(theme, label)` — a bordered group box with a heading — containing Base URL / API Key / Model fields. The Save button is `button("btn-save", text!("Save")).primary(&theme)` — inverted colors (theme text as background, theme bg as text), converted with `.into_element()` to attach `on_click` (see [`button()` — Themed Buttons](#button--themed-buttons)).

#### Gotcha: `text!` Does Not Format

gpui's `text!` macro does **not** run format strings — `text!("{label}")` renders the literal `{label}`. Pass the expression directly instead: `text!(label)`. This is why the `field`/`model_group` helpers take `label: &str` and render `text!(label)`.

## Theme (`gui/src/theme.rs`)

```rust
pub struct Theme {
  kind: ThemeKind,   // serde(skip)
  pub bg: Rgba,
  pub text: Rgba,
  pub label: Rgba,       // label text (orange in the dark theme)
  pub border: Rgba,      // borders at 25% opacity
  pub danger_bg: Rgba,   // destructive UI background (dark red)
  pub danger_fg: Rgba,   // destructive UI foreground (lighter red than danger_bg)
  pub success: Rgba,     // positive feedback accent (soft green)
  pub warn: Rgba,        // warning feedback accent (amber)
}
```

- `Theme::dark()` constructor (bg `#283333`, text `#E1F5F5`, label `#E87813`, border `#666` @ 25%, danger_bg `#642C2C`, danger_fg `#E88C8C`, success `#5FA867`, warn `#E8B813`); `Default` delegates to `dark()`
- `ThemeKind` is a unique-identifier enum (`Dark`) with `FromStr` parsing
- Implements gpui's `Global` trait — screens read it via `cx.global::<Theme>()`; `main()` installs it with `cx.set_global(config.theme)`
- Serialized/deserialized as the theme name string (`"dark"`) via `serialize_theme_name`/`deserialize_theme_name`; the GUI config lives at `{configDir}/NovelCraft/gui.config.json`

## Styling Conventions

### Tailwind-Like API

All styling uses gpui's `Styled` trait, which provides Tailwind-like methods on any element implementing it. These are a mix of hand-written methods in `styled.rs` and macro-generated methods from `gpui_macros`.

### Cursor

```rust
.cursor_pointer()                          // CursorStyle::PointingHand
.cursor(CursorStyle::IBeam)                // generic form
.cursor_text()                             // CursorStyle::IBeam
.cursor_not_allowed()                      // CursorStyle::OperationNotAllowed
```

### Hover Styles

```rust
// CSS-like hover refinement on InteractiveElement
.hover(|style| style.opacity(0.7))

// Hover callback on StatefulInteractiveElement (bool: true=enter, false=leave)
.on_hover(|&is_hovering: &bool, window, cx| { ... })
```

### Positioning

```rust
.relative()
.absolute()
.top_4()          // 1rem (16px)
.right_4()         // 1rem (16px)
.top(px(8.))       // custom pixel value
.inset_0()         // all sides 0px
```

### Event Handlers

```rust
// On InteractiveElement (no id required)
.on_mouse_down(MouseButton::Left, |ev, window, cx| { ... })
.on_mouse_up(MouseButton::Left, |ev, window, cx| { ... })
.on_mouse_move(|ev, window, cx| { ... })
.on_mouse_exit(|ev, window, cx| { ... })

// On StatefulInteractiveElement (requires .id() first)
.on_click(|ev: &ClickEvent, window, cx| { ... })
.on_hover(|&is_hovering: &bool, window, cx| { ... })
```

### Dispatching Actions

Inside event handlers (`on_click`, `on_mouse_down`, etc.), always dispatch actions via `Window::dispatch_action`, never `App::dispatch_action`. During event handling the window is mid-update (its slot in `cx.windows` is temporarily taken), so the app-level synchronous re-entrant dispatch fails and logs "window not found". `Window::dispatch_action` defers the dispatch via `cx.defer` — this is the pattern Zed's own UI components use.

```rust
// Correct: window-scoped dispatch (defers via cx.defer)
.on_click(|_, window, cx| window.dispatch_action(ShowSettings.boxed_clone(), cx))

// Wrong: app-level dispatch inside an event handler ("window not found")
.on_click(|_, _, cx| cx.dispatch_action(&ShowSettings))
```

`App::dispatch_action` is reserved for app-level/global dispatch outside window updates (e.g. from timers or menus).

Navigation actions (`Back`, `ShowSettings`, `CreateStory`, `ShowStory`, `PlayStory`, `PagePrev`, `PageNext`) are handled by app-level global listeners registered once in `main()`. These listeners receive actions in the bubble phase after no element handler consumes them, and run both for actions dispatched through a window (`Window::dispatch_action`) and for global dispatches (`App::dispatch_action`). `AppRoot` is created via `cx.new` before `open_window` and passed into the window as its root view. The generic helper `on_screen_action(cx, root, to)` registers a listener via `App::on_action` that sets `root.screen` to the `Screen` returned by the mapping closure — the trivial navigation actions each get a one-line registration. `Back` is the exception: its target depends on the current screen, so it gets an explicit `cx.on_action` forwarding to the `&mut self` method `AppRoot::on_back`; `PagePrev`/`PageNext` are similarly explicit, forwarding to `AppRoot::gameplay_nav` (a no-op unless the gameplay screen is showing).

```rust
let root = cx.new(|cx| AppRoot { /* ... */ });

// Back's target depends on the current screen — explicit handler:
cx.on_action({
  let root = root.clone();
  move |_: &actions::Back, cx| root.update(cx, |root, _cx| root.on_back())
});

// Trivial navigation — one line per action:
on_screen_action(cx, &root, |_: &actions::ShowSettings| Screen::Settings);
on_screen_action(cx, &root, |_: &actions::CreateStory| Screen::CreateStory);
on_screen_action(cx, &root, |ev: &actions::ShowStory| {
  Screen::StoryOverview(ev.0.clone())
});
on_screen_action(cx, &root, |ev: &actions::PlayStory| {
  Screen::StoryGameplay(ev.0.clone())
});

// Page navigation — bound globally to alt-left / alt-right (no key context):
cx.bind_keys([
  KeyBinding::new("alt-left", actions::PagePrev, None),
  KeyBinding::new("alt-right", actions::PageNext, None),
]);
cx.on_action({
  let root = root.clone();
  move |_: &actions::PagePrev, cx| root.update(cx, |root, cx| root.gameplay_nav(-1, cx))
});
cx.on_action({
  let root = root.clone();
  move |_: &actions::PageNext, cx| root.update(cx, |root, cx| root.gameplay_nav(1, cx))
});

cx.open_window(WindowOptions::default(), move |_, _| root.clone()).unwrap();
```

### Callbacks with View State

Use `cx.listener()` to bind action/element handlers to view methods — the listener captures `&mut Self` of the owning view:

```rust
// In TextInput's Render impl — each action routes to a &mut self method:
.on_action(cx.listener(Self::backspace))
.on_action(cx.listener(Self::submit_action))

// Or inline closures:
.on_click(cx.listener(|this: &mut Self, ev: &ClickEvent, window, cx| {
  this.screen = Screen::Home;
  cx.notify();
}))
```

### Views as Entities

Views that appear in multiple places, or that need to outlive a single render, are created once via `cx.new` and stored as `Entity<T>`:

```rust
// Screens are created once and stored on AppRoot:
screen_home: cx.new(|cx| HomeScreen::create(cx)),
// Render attaches a clone of the entity:
res = res.child(self.screen_home.clone());
```

Inside a view's own `Render` impl, `cx.entity()` hands the entity to a custom element — this is how `TextInput` passes itself to `TextElement { input: cx.entity() }`.

## File Organization

```
gui/src/
├── main.rs          # Entry point — engine thread, CommandBus/AppEvents channels, EngineQuery globals, AppRoot view, action routing
├── screens/           # Screen enum (mod.rs) + one submodule per screen (create(cx) + Render)
├── comp.rs          # Stateless UI builders (root, screen_root, top_bar, settings_gear, btn_icon_close)
├── text_input.rs    # Reusable TextInput component (custom Element, IME, scoped key bindings)
├── theme.rs         # Theme/ThemeKind (bg, text colors), Global impl, serde as theme name
└── util.rs          # Loggable trait, LogLevel
```

## Related Documentation

- [Project Structure](./project-structure.md) — Overall file organization
- [Code Conventions](./code-conventions.md) — Coding standards
- [Engine API](./api-routes.md) — Engine command function reference
- [Data Storage](./database-schema.md) — JSON file formats and storage layout
