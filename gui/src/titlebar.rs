#[cfg(not(target_arch = "wasm32"))]
use dioxus::desktop::tao::window::ResizeDirection;
#[cfg(not(target_arch = "wasm32"))]
use dioxus::desktop::use_window;
use dioxus::prelude::*;

/// Custom window chrome, replacing the platform-default title bar (the
/// window is launched with native decorations disabled). The whole strip
/// is a drag handle; double-click toggles maximize. The window control
/// buttons stop propagation so they don't trigger the drag.
#[cfg(not(target_arch = "wasm32"))]
#[component]
pub fn TitleBar() -> Element {
  // DesktopContext is a cheap Rc handle — clone one per event closure.
  let w_drag = use_window();
  let w_max = w_drag.clone();
  let w_min = w_drag.clone();
  let w_toggle = w_drag.clone();
  let w_close = w_drag.clone();

  rsx! {
    div {
      class: "flex-none h-8 flex items-stretch bg-nc-bg border-b border-nc-border select-none",
      onmousedown: move |_| w_drag.drag(),
      ondoubleclick: move |_| w_max.toggle_maximized(),

      div { class: "flex-1" }

      button {
        class: "w-10 flex items-center justify-center text-sm text-nc-text hover:bg-nc-text/10",
        onmousedown: move |ev| ev.stop_propagation(),
        ondoubleclick: move |ev| ev.stop_propagation(),
        onclick: move |_| w_min.window.set_minimized(true),
        "\u{2013}"
      }
      button {
        class: "w-10 flex items-center justify-center text-xs text-nc-text hover:bg-nc-text/10",
        onmousedown: move |ev| ev.stop_propagation(),
        ondoubleclick: move |ev| ev.stop_propagation(),
        onclick: move |_| w_toggle.toggle_maximized(),
        "\u{25a1}"
      }
      button {
        class: "w-10 flex items-center justify-center text-base text-nc-text hover:bg-nc-danger hover:text-nc-bg",
        onmousedown: move |ev| ev.stop_propagation(),
        ondoubleclick: move |ev| ev.stop_propagation(),
        onclick: move |_| w_close.close(),
        "\u{00d7}"
      }
    }
  }
}

/// Invisible edge & corner strips that restore window resizing, which the
/// window manager suspends for undecorated windows. Rendered once, above
/// all content.
#[cfg(not(target_arch = "wasm32"))]
#[component]
pub fn ResizeHandles() -> Element {
  let w = use_window();

  rsx! {
    for (dir, class, cursor) in HANDLES {
      {
        let w = w.clone();
        rsx! {
          div {
            key: "{dir:?}",
            class: "fixed z-100 {class} {cursor}",
            onmousedown: move |_| {
              let _ = w.window.drag_resize_window(dir);
            },
          }
        }
      }
    }
  }
}

/// (direction, fixed-position classes, cursor) for each of the 8 resize
/// zones. Edges are 4px thick, corners 12px.
#[cfg(not(target_arch = "wasm32"))]
const HANDLES: [(ResizeDirection, &str, &str); 8] = [
  (ResizeDirection::North, "top-0 left-0 right-0 h-1", "cursor-ns-resize"),
  (ResizeDirection::South, "bottom-0 left-0 right-0 h-1", "cursor-ns-resize"),
  (ResizeDirection::West, "top-0 bottom-0 left-0 w-1", "cursor-ew-resize"),
  (ResizeDirection::East, "top-0 bottom-0 right-0 w-1", "cursor-ew-resize"),
  (ResizeDirection::NorthWest, "top-0 left-0 w-3 h-3", "cursor-nwse-resize"),
  (ResizeDirection::SouthEast, "bottom-0 right-0 w-3 h-3", "cursor-nwse-resize"),
  (ResizeDirection::NorthEast, "top-0 right-0 w-3 h-3", "cursor-nesw-resize"),
  (ResizeDirection::SouthWest, "bottom-0 left-0 w-3 h-3", "cursor-nesw-resize"),
];

/// Web builds render nothing — the browser provides its own window chrome.
#[cfg(target_arch = "wasm32")]
#[component]
pub fn TitleBar() -> Element {
  rsx! {}
}

/// Web builds render nothing — the browser provides its own resize frame.
#[cfg(target_arch = "wasm32")]
#[component]
pub fn ResizeHandles() -> Element {
  rsx! {}
}
