# GUI Architecture

This document describes the gpui-based GUI crate (`gui/`, binary `novelcraft-gui`).

## Overview

The GUI is a native Rust binary using the **gpui** framework (from the Zed editor repo) for rendering. There is no HTML, no CSS, no JavaScript. Screens are plain Rust functions returning `Div` elements. Navigation between screens is handled by an enum-based state machine in the root view.

### Key Technologies

- **gpui** (git, Zed main) — UI framework: views, elements, Tailwind-like styling via the `Styled` trait
- **gpui_platform** (git, Zed main) — Platform integration: window management, app lifecycle
- **novelcraft-engine** (path) — Business logic library for data persistence, LLM proxy, game loop

## Architecture

### Screen-Based Navigation

The `Screen` enum (`gui/src/screens/mod.rs`) drives navigation. `AppRoot` matches on the current `Screen` variant and delegates to the corresponding screen's `render()` function.

```rust
pub enum Screen {
  Gameplay,
  Home(home::HomeData),
  Settings(Theme),
  Story,
}
```

Each screen module exposes a `pub fn render(...) -> Div` function. Screens are not gpui `View` structs — they are stateless rendering functions that return styled `Div` element trees. State lives in `AppRoot` (or engine-level persistence).

### Screen Functions

| Screen | Module | Signature |
|--------|--------|-----------|
| Home | `screens::home` | `render(data: &HomeData, gear: impl IntoElement) -> Div` |
| Settings | `screens::settings` | `render(theme: &Theme, on_close: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static) -> Div` |
| Gameplay | `screens::gameplay` | `render() -> Div` |
| Story | `screens::story` | `render() -> Div` |

### AppRoot View

`AppRoot` (`gui/src/main.rs`) is the sole top-level gpui `View`. It holds:

- `screen: Screen` — current screen variant
- `settings_gear: Entity<SettingsGear>` — the gear icon as a gpui entity, shared across renders
- `rx_chunks` / `tx_cmds` — channels for engine communication

The `Render` impl matches on `self.screen` and calls the appropriate screen function. Navigation is performed by mutating `self.screen` and calling `cx.notify()`.

## Reusable Components (`gui/src/comp.rs`)

### `root(theme: &Theme) -> Div`

Returns a `Div` pre-configured with the theme's background and text color. All screen render functions wrap their content in `root(&data.theme)` for consistent theming.

### `SettingsGear` (gpui View)

A `pub(crate)` gpui `View` that renders a gear icon (Unicode `\u{2699}`, ⚙) with interactive behavior.

**Construction:**

```rust
SettingsGear::new(|_ev: &ClickEvent, _window: &mut Window, cx: &mut App| {
    // navigation logic
})
```

**Behavior:**

- Renders `"⚙"` at `text_xl()` size
- Cursor changes to pointer on hover (`cursor_pointer()`)
- Opacity drops to 0.7 on hover (`hover(|style| style.opacity(0.7))`)
- Click invokes the callback provided at construction time
- The callback is wrapped in `Arc` internally to satisfy gpui's `'static` requirements
- Requires an `.id("settings-gear")` for interactivity (stateful element)

**Usage pattern:** `SettingsGear` is instantiated as a gpui `Entity` in `AppRoot`'s constructor. The click callback captures a weak reference to `AppRoot` and mutates the screen state. The entity is passed to `screens::home::render()` as an `impl IntoElement` argument.

## Screen Details

### Home Screen (`gui/src/screens/home.rs`)

Displays the app title and a settings gear icon.

**Signature:**

```rust
pub fn render(data: &HomeData, gear: impl IntoElement) -> Div
```

**Parameters:**

| Parameter | Type | Description |
|-----------|------|-------------|
| `data` | `&HomeData` | Contains `theme: Theme` |
| `gear` | `impl IntoElement` | The settings gear element (typically an `Entity<SettingsGear>`) |

**Layout:**

- Root: `root(&data.theme)`, positioned `relative()`, flex column, items centered
- Heading row: `.flex().flex_row().justify_between().w_full()` — places the title left-aligned and the gear icon right-aligned
- App title: `"NovelCraft"` at `text_3xl()` (left child of heading row)
- Gear icon: right child of heading row (the `impl IntoElement` parameter)

**Navigation:** Clicking the gear icon triggers the `on_click` callback defined in `SettingsGear::new()`, which transitions `AppRoot.screen` from `Screen::Home(data)` to `Screen::Settings(data.theme.clone())`.

### Settings Screen (`gui/src/screens/settings.rs`)

Displays a settings header with a close button.

**Signature:**

```rust
pub fn render(
    theme: &Theme,
    on_close: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Div
```

**Parameters:**

| Parameter | Type | Description |
|-----------|------|-------------|
| `theme` | `&Theme` | Theme for background and text colors |
| `on_close` | `impl Fn(&ClickEvent, &mut Window, &mut App) + 'static` | Callback invoked when the close button is clicked |

**Layout:**

- Root: `root(theme)`, positioned `relative()`, flex column, items centered
- Title: `"NovelCraft Settings"` at `text_2xl()`
- Close button: absolute-positioned top-right (`.absolute().top_4().right_4()`), renders `"×"` (Unicode `\u{00d7}`) at `text_xl()` with `cursor_pointer()` and hover opacity 0.7

**Navigation:** The `on_close` callback is provided by `AppRoot` via `cx.listener()`. It clones the theme, sets `self.screen = Screen::Home(HomeData { theme })`, and calls `cx.notify()`.

## Theme (`gui/src/theme.rs`)

```rust
pub struct Theme {
    pub bg: Rgba,
    pub text: Rgba,
}
```

Provides a `dark()` constructor (bg `#283333`, text `#E1F5F5`) and implements `Default` by delegating to `dark()`.

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

### Callbacks with View State

Use `cx.listener()` to capture view state in callbacks:

```rust
let on_close = cx.listener(move |this: &mut AppRoot, _ev: &ClickEvent, _window, cx| {
    this.screen = Screen::Home(Default::default());
    cx.notify();
});
screens::settings::render(theme, on_close)
```

### Entity for Shared Callbacks

When a component with a `'static` callback needs to be passed as `impl IntoElement`, wrap it in an `Entity`:

```rust
let gear = cx.new(|_| {
    SettingsGear::new(move |ev, window, cx| { /* ... */ })
});
// Later, in render:
screens::home::render(&data, gear.clone())
```

## File Organization

``
gui/src/
├── main.rs          # Entry point, AppRoot view, screen dispatch
├── comp.rs          # Reusable components (root(), SettingsGear)
├── theme.rs         # Theme struct (bg, text colors)
├── util.rs          # Loggable trait, LogLevel
└── screens/
    ├── mod.rs       # Screen enum, Default impl
    ├── home.rs      # Home screen render function
    ├── settings.rs  # Settings screen render function
    ├── gameplay.rs  # Gameplay screen (placeholder)
    └── story.rs     # Story screen (placeholder)
```

## Related Documentation

- [Project Structure](./project-structure.md) — Overall file organization
- [Code Conventions](./code-conventions.md) — Coding standards
- [Engine API](./api-routes.md) — Engine command function reference
- [Data Storage](./database-schema.md) — JSON file formats and storage layout
