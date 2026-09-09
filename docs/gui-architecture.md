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
| Home | `HomeScreen` | `sessions: Option<Vec<SessionV1>>` (see [Home Screen](#home-screen-homescreen)) |
| Settings | `SettingsScreen` | `config: Option<NovelCraftConfig>` + `Entity<TextInput>` per editable field (see [Settings Screen](#settings-screen-settingsscreen)) |
| Create Story | `CreateStoryScreen` | `title` / `premise`: `Entity<TextInput>` (see [Create Story Screen](#create-story-screen-createstoryscreen)) |
| Story Overview | `StoryOverviewScreen` | `id: StoryId` |
| Story Gameplay | `StoryGameplayScreen` | `id: StoryId` |

Each screen lives in its own submodule under `gui/src/screens/` (`home.rs`, `settings.rs`, `create_story.rs`, `story_overview.rs`, `story_gameplay.rs`); `mod.rs` declares the submodules, defines the `Screen` enum, and re-exports the screen structs. `StoryOverviewScreen` and `StoryGameplayScreen` are currently placeholders (`render` returns an empty `div()`).

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
- `rx_chunks` — `mpsc::Receiver<AgentMessageChunk>` for engine communication (the `mpsc::Sender` side lives in the `CommandBus` global and in the engine thread)

The `Render` impl matches on `self.screen` and attaches the matching screen entity as a child. For `StoryOverview`/`StoryGameplay` it first syncs the `StoryId` into the screen entity via `cx.update_entity`. Navigation is performed by mutating `self.screen` (global action listeners do this via `root.update(cx, ...)`); gpui re-renders automatically since the entities are observed.

Engine communication: a `CommandBus(mpsc::Sender<Command>)` (both `pub(crate)`) is registered as a gpui `Global`; a dedicated engine thread runs a tokio runtime and consumes the receiving end. `Command` is a `pub(crate)` enum with six variants:

| Variant | Payload | Semantics |
|---------|---------|-----------|
| `SwitchProfile(String)` | profile ID | Fire-and-forget: `engine.set_active_profile(Some(id))` |
| `Prompt(String)` | user prompt | Fire-and-forget: `engine.prompt(...)`, chunks streamed over the `mpsc` chunk channel |
| `LoadConfig(oneshot::Sender<NovelCraftConfig>)` | reply channel | Request/response: engine thread loads `{configDir}/NovelCraft/config.json` via `NovelCraftConfig::load()` (falling back to `NovelCraftConfig::default()` on error, logged as a warning), syncs it via `NovelCraftEngine::set_config`, replies over the channel |
| `SaveConfig(Box<NovelCraftConfig>)` | boxed config | Fire-and-forget: engine thread syncs via `engine.set_config`, then persists with `config.save().await` |
| `ListSessions(oneshot::Sender<Vec<SessionV1>>)` | reply channel | Request/response: engine thread calls `NovelCraftEngine::list_sessions()` — enumerates session directories, loads each via `SessionV1::load_metadata` (metadata only: id, title, exposition, timestamps, modules, profile; no page batches or gamestate), skips directories with unreadable metadata (logged as a warning), sorts newest-first by `updated_at`, and replies; on error it logs a warning and replies with an empty `Vec` |
| `CreateSession { title, exposition, reply }` | title, exposition, reply channel | Request/response: engine thread calls `NovelCraftEngine::create_session(title, exposition)` — builds a fresh `SessionV1` with default gameplay modules and the engine's active profile, persists it — and replies `Some(session)`; on error it logs the error and replies `None` |

`Command` derives nothing — the oneshot sender field is neither `Debug` nor `Clone`. Commands are sent via `CommandBus::send` (a `blocking_send` whose error is logged through the `Loggable` trait). The request/response pattern (`LoadConfig`, `ListSessions`, `CreateSession`) works by having the caller create a `tokio::sync::oneshot` channel, pass the `Sender` in the command, and await the `Receiver` inside a `cx.spawn`ed (detached) future — see [Settings Screen](#settings-screen-settingsscreen), [Home Screen](#home-screen-homescreen), and [Create Story Screen](#create-story-screen-createstoryscreen).

## Reusable Components (`gui/src/comp.rs`)

`comp.rs` contains **stateless builder functions** — plain functions returning styled elements. They hold no state and register no key bindings.

| Function | Returns | Purpose |
|----------|---------|---------|
| `root(theme: &Theme)` | `Div` | Root of most screens: flex column, items centered, theme bg + text color |
| `screen_root()` | `Div` | Screen content wrapper: flex column, items centered, `w_full h_full` |
| `content()` | `Stateful<Div>` | Scrollable content column: `w_full`, `max_w(px(640.))`, `gap_4`, `p_4` |
| `top_bar()` | `Div` | Title bar row: `relative w_full`, flex row, `justify_center` |
| `title(content: Text)` | `Div` | Screen title text at `text_3xl()` |
| `field(theme, label, input)` | `Div` | Labeled form field: label above an `Entity<TextInput>` |
| `field_group(theme, label)` | `Div` | Bordered group box with a heading (used for model groups) |
| `create_text_input(cx, multiline, placeholder)` | `Entity<TextInput>` | Creates a `TextInput` entity with the given mode and placeholder |
| `button(anim_id, label)` | `IncompleteButton` | Starts a themed `Button` (see below) |
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

`HomeScreen` holds a single field, `sessions: Option<Vec<SessionV1>>`. `create(cx)` immediately calls `load(cx)`, which sends `Command::ListSessions` via the `CommandBus` using the same oneshot + `cx.spawn` request/response pattern as `SettingsScreen::load` — the spawned future awaits the reply and stores it in `self.sessions` via `populate(...)`, then `cx.notify()` triggers a re-render.

#### Render

Below the top bar, a `content()` column renders:

- **"Create new Vignette" card** — a jumbo full-width clickable bordered card (`create_vignette(theme)` helper): a horizontally+vertically centered title row (`flex().items_center().justify_center().gap_2()`) with a "+" text glyph before the "Create new Vignette" text, both at `text_2xl()`, plus the description that Vignettes are free-form stories that don't follow any template, resembling other story generator platforms. Hover swaps the border color to the label color; clicking dispatches the existing `nav::CreateStory` action via `window.dispatch_action`.
- **"Sessions" section** — a `text_xl()` heading followed by the sessions list (`sessions_ui`), which has three states:
  - `None` (loading) — a "Loading ..." placeholder pulsating via gpui's animation API: `with_animation(..., Animation::new(1s).repeat().with_easing(pulsating_between(0.2, 1.0)), |el, delta| el.opacity(delta))`
  - `Some([])` — the static text "No sessions yet."
  - `Some(sessions)` — a column (gap-2) of clickable session cards

Each session card is built by the `session_card(theme, session)` helper: a bordered, hover-highlighted, clickable row showing the session title (`text_lg`), the `updated_at` timestamp formatted in local time via `chrono` (`%Y-%m-%d %H:%M`), and the exposition text. Cards get their session ID as the element ID; `on_click` dispatches `nav::ShowStory(StoryId)` where `StoryId` wraps the session ID. Both card types dispatch via `window.dispatch_action` and navigate through the app-level action listeners in `main()`.

### Create Story Screen (`CreateStoryScreen`)

The vignette creation form, reached from the Home screen's "Create new Vignette" card (`nav::CreateStory` → `Screen::CreateStory`). The top bar shows the title `"Create Vignette"` at `text_3xl()` (centered) with `btn_icon_close()` (`Back` → `Screen::Home`).

#### Fields

| Screen field | Type | Editor |
|--------------|------|--------|
| `title` | `Entity<TextInput>` | single-line, placeholder `"Title"` |
| `premise` | `Entity<TextInput>` | multiline, placeholder `"Describe the premise your story starts from ..."` |

Both inputs are created with the `create_text_input(cx, multiline, placeholder)` comp helper and rendered via `field(theme, label, input)`. The Create button is a `button("btn-create", text!("Create")).primary(&theme)` — identical to the Settings save button (see [`button()` — Themed Buttons](#button--themed-buttons)).

#### Submit Flow

`submit()` (bound to the Create button via `cx.listener`) reads both values trimmed and no-ops when the title is empty. Otherwise it sends `Command::CreateSession { title, exposition, reply }` over the `CommandBus` and, inside a `cx.spawn`ed (detached) task, awaits the oneshot reply. On `Some(session)` it dispatches the `nav::ShowStory(StoryId(session.id))` action via `App::dispatch_action` — allowed here because the spawned future runs outside a window update (note that `AsyncApp::update` returns the closure result directly, not a `Result`). The app-level listener routes the app to `Screen::StoryOverview`.

Known limitation (deferred): input values are not reset when re-entering the screen — the screen entity is created once at startup and reused.

### Settings Screen (`SettingsScreen`)

A functional editor for the engine's `NovelCraftConfig` (persisted at `{configDir}/NovelCraft/config.json`). The top bar keeps the title `"Settings"` at `text_3xl()` (centered) and `btn_icon_close()` (absolute top-right); clicking `×` dispatches `Back`, which `AppRoot::on_back` maps to `Screen::Home` (or `Screen::StoryOverview` when returning from gameplay).

#### Fields

| Screen field | Type | Editor |
|--------------|------|--------|
| `config` | `Option<NovelCraftConfig>` | — (populated by `load()`, kept as the save base) |
| `max_agent_steps` | `Entity<TextInput>` | single-line, placeholder `"10"` |
| `system_prompt` | `Entity<TextInput>` | multiline, no placeholder |
| `models` | `Vec<ModelFields>` | two groups (`const MODEL_GROUPS: [&str; 2] = ["Dungeon Master", "Suggestions"]`) |

`ModelFields` holds one `Entity<TextInput>` per `ModelConfig::OpenAi` field — `base_url` (placeholder `DEFAULT_HOST`, i.e. `http://localhost:8888/v1`), `api_key`, and `model`. All inputs are built in a module-private `text_input(cx, multiline, placeholder)` helper; `create(cx)` assembles the screen and immediately calls `load(cx)`.

#### Load Flow (Oneshot Request/Response)

```rust
fn load(&mut self, cx: &mut Context<'_, Self>) {
  let (tx, rx) = oneshot::channel();
  cx.global::<CommandBus>().send(Command::LoadConfig(tx));
  cx.spawn(async move |this, cx| {
    if let Ok(config) = rx.await
      && let Ok(()) = this.update(cx, |screen, cx| screen.populate(config, cx))
    {}
  })
  .detach();
}
```

`Command::LoadConfig` is handled on the engine thread (see the command table under [Architecture](#architecture)): it loads `{configDir}/NovelCraft/config.json` via `NovelCraftConfig::load()`, falls back to `NovelCraftConfig::default()` on error (logged as a warning), syncs the config into the engine via `NovelCraftEngine::set_config(config.clone())`, then replies over the oneshot channel. The screen's spawned future awaits the reply and populates the inputs via `this.update(cx, ...)`; the future is detached.

`populate(config, cx)` fills every input via `TextInput::set_value` (`max_agent_steps` via `to_string()`, model fields destructured from the `ModelConfig::OpenAi` variants), stores the config in `self.config` — preserving `profiles` and `active_profile` for the later save — and calls `cx.notify()`.

#### Save Flow

`save()` is bound to the Save button via `cx.listener`. It clones `self.config` (`unwrap_or_default()` if the load never completed) and rebuilds the editable parts from the inputs:

- `max_agent_steps` — input text trimmed and parsed as `u8`; falls back to the previous value when unparsable (the input is not restricted to numeric text yet)
- `system_prompt` — raw input text
- Model groups — each `ModelFields` mapped to `ModelConfig::OpenAi { base_url, api_key, model }` (base URL and model trimmed) via the `model_config()` helper
- `profiles` / `active_profile` — **not editable**; carried over untouched from the stored config

The result is boxed and sent as `Command::SaveConfig(Box<NovelCraftConfig>)`. The engine thread syncs it via `engine.set_config(*config.clone())` and persists it with `config.save().await` (errors logged through `Loggable`).

#### Layout

`screen_root()` with the top bar plus a scrollable content column: `.id("settings-scroll").overflow_y_scroll()` (`flex_grow(1.)`, `min_h(px(0.))`, `w_full`) wrapping an inner column with `max_w(px(640.))`, `gap_4`, `p_4`. Fields are rendered by the `field(label, input, color)` helper (label at 55% text opacity above the input). Each model entry is a `model_group(label, fields, color, border)` bordered box (`border_1`, 25% opacity border, `rounded_sm`, `p_3`) containing Base URL / API Key / Model fields. The Save button is `button("btn-save", text!("Save")).primary(&theme)` — inverted colors (theme text as background, theme bg as text), converted with `.into_element()` to attach `on_click` (see [`button()` — Themed Buttons](#button--themed-buttons)).

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
}
```

- `Theme::dark()` constructor (bg `#283333`, text `#E1F5F5`, label `#E87813`, border `#666` @ 25%, danger_bg `#642C2C`, danger_fg `#E88C8C`); `Default` delegates to `dark()`
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

Navigation actions (`Back`, `ShowSettings`, `CreateStory`, `ShowStory`, `PlayStory`) are handled by app-level global listeners registered once in `main()`. These listeners receive actions in the bubble phase after no element handler consumes them, and run both for actions dispatched through a window (`Window::dispatch_action`) and for global dispatches (`App::dispatch_action`). `AppRoot` is created via `cx.new` before `open_window` and passed into the window as its root view. The generic helper `on_screen_action(cx, root, to)` registers a listener via `App::on_action` that sets `root.screen` to the `Screen` returned by the mapping closure — the four trivial navigation actions each get a one-line registration. `Back` is the exception: its target depends on the current screen, so it gets an explicit `cx.on_action` forwarding to the `&mut self` method `AppRoot::on_back`:

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
├── main.rs          # Entry point — engine thread, CommandBus global, AppRoot view, action routing
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
