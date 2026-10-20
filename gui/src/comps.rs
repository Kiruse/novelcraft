use dioxus::prelude::*;

use crate::markdown::{self, Block, Segment};
use crate::screens::{go_back, Route};

/// Shared class list for text inputs.
pub const INPUT_CLASS: &str =
  "w-full px-3 py-2 bg-transparent border border-nc-border rounded-sm text-nc-text \
   focus:border-nc-label focus:outline-none placeholder:text-nc-text/40 resize-none";

#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(dead_code)]
pub enum BtnVariant {
  Primary,
  Secondary,
  DangerPrimary,
  DangerSecondary,
}

impl BtnVariant {
  fn classes(&self) -> &'static str {
    match self {
      Self::Primary => {
        "bg-nc-text text-nc-bg border-transparent hover:bg-nc-text/85 active:brightness-90"
      }
      Self::Secondary => {
        "bg-transparent text-nc-text border-nc-text hover:bg-nc-text/10 active:bg-nc-text/5"
      }
      Self::DangerPrimary => {
        "bg-nc-danger text-nc-bg border-transparent hover:brightness-110 active:brightness-90"
      }
      Self::DangerSecondary => {
        "bg-transparent text-nc-dangerfg border-nc-dangerfg hover:brightness-110 hover:bg-nc-dangerfg/10 active:bg-nc-dangerfg/20"
      }
    }
  }
}

#[derive(Props, Clone, PartialEq)]
pub struct ButtonProps {
  label: String,
  variant: BtnVariant,
  #[props(default)]
  disabled: bool,
  on_click: EventHandler,
}

#[component]
pub fn Button(props: ButtonProps) -> Element {
  let disabled_classes = if props.disabled {
    "bg-nc-text/10 text-nc-text/90 border-transparent cursor-default"
  } else {
    "cursor-pointer"
  };

  rsx! {
    button {
      class: "w-full p-2 flex justify-center rounded-sm border {props.variant.classes()} {disabled_classes}",
      disabled: props.disabled,
      onclick: move |_| props.on_click.call(()),
      { props.label }
    }
  }
}

#[derive(Props, Clone, PartialEq)]
pub struct ChipProps {
  label: String,
  selected: bool,
  on_click: EventHandler,
}

#[component]
pub fn Chip(props: ChipProps) -> Element {
  let state_classes = if props.selected {
    "bg-nc-text text-nc-bg border-transparent hover:bg-nc-text/85"
  } else {
    "bg-transparent text-nc-text border-nc-border hover:bg-nc-text/10 hover:border-nc-label"
  };

  rsx! {
    button {
      class: "px-2 py-0.5 text-sm rounded-full border cursor-pointer {state_classes}",
      onclick: move |_| props.on_click.call(()),
      { props.label }
    }
  }
}

#[derive(Props, Clone, PartialEq)]
pub struct FieldProps {
  label: String,
  children: Element,
}

/// Label above an input, spanning the full width.
#[component]
pub fn Field(props: FieldProps) -> Element {
  rsx! {
    div { class: "flex flex-col gap-1 w-full",
      div { class: "text-nc-label", { props.label } }
      { props.children }
    }
  }
}

#[derive(Props, Clone, PartialEq)]
pub struct FieldGroupProps {
  label: String,
  children: Element,
}

/// Bordered group of fields with a heading label.
#[component]
pub fn FieldGroup(props: FieldGroupProps) -> Element {
  rsx! {
    div { class: "flex flex-col gap-2 w-full p-3 border rounded-sm border-nc-border",
      div { class: "text-lg", { props.label } }
      { props.children }
    }
  }
}

#[component]
pub fn Loading(text: String) -> Element {
  rsx! {
    div { class: "animate-pulse", { text } }
  }
}

/// Markdown-like text rendering (see [`crate::markdown`]) — a streaming-
/// safe subset: paragraphs, headings, blockquotes, `**bold**`, `*italic*`
/// and double-quoted `"speech"`, which breaks onto its own line in white
/// while all other text renders at 80% opacity (`.markdown` in input.css).
/// Incomplete markers (open bold/italic/quote at end-of-text) render
/// as-if closed, so streamed agent output never leaks raw syntax.
#[derive(Props, Clone, PartialEq)]
pub struct MarkdownProps {
  text: String,
  #[props(default)]
  class: String,
}

#[component]
pub fn Markdown(props: MarkdownProps) -> Element {
  let blocks = markdown::parse(&props.text);
  let class = format!("markdown {}", props.class);

  rsx! {
    div { class, { render_blocks(&blocks) } }
  }
}

fn render_blocks(blocks: &[Block]) -> Element {
  rsx! {
    for (idx, block) in blocks.iter().enumerate() {
      { match block {
          Block::Heading { level, segments } => rsx! {
            div { key: "{idx}", class: "md-h md-h{level}", { render_segments(segments) } }
          },
          Block::Quote(inner) => rsx! {
            blockquote { key: "{idx}", { render_blocks(inner) } }
          },
          Block::Paragraph(segments) => rsx! {
            p { key: "{idx}", { render_segments(segments) } }
          },
      } }
    }
  }
}

fn render_segments(segments: &[Segment]) -> Element {
  rsx! {
    for (idx, segment) in segments.iter().enumerate() {
      { match segment {
          Segment::Speech(children) => rsx! {
            span { key: "{idx}", class: "md-speech", { render_segments(children) } }
          },
          Segment::Styled { style, text } => rsx! {
            span {
              key: "{idx}",
              class: markdown::styled_classes(*style),
              { text.clone() }
            }
          },
      } }
    }
  }
}

/// Standard screen layout: centered top bar with title + (gear or close),
/// above a centered, scrollable content column.
#[derive(Props, Clone, PartialEq)]
pub struct ScreenProps {
  title: String,
  #[props(default)]
  closable: bool,
  children: Element,
}

#[component]
pub fn Screen(props: ScreenProps) -> Element {
  rsx! {
    div { class: "flex flex-col items-center w-full h-full",
      div { class: "relative w-full flex flex-row justify-center",
        h1 { class: "text-3xl", { props.title } }
        { if props.closable {
            rsx! { CloseButton {} }
          } else {
            rsx! { SettingsGear {} }
          }
        }
      }
      // Full-width scroller — the scrollbar sits at the window edge; the
      // content column is centered inside it.
      div { class: "flex-1 min-h-0 w-full overflow-y-auto",
        div { class: "relative flex flex-col items-center w-full max-w-160 mx-auto gap-4 p-4",
          { props.children }
        }
      }
    }
  }
}

#[component]
pub fn SettingsGear() -> Element {
  let nav = navigator();
  rsx! {
    button {
      class: "group absolute right-4 p-1 cursor-pointer text-nc-text hover:opacity-70",
      onclick: move |_| {
        nav.push(Route::Settings {});
      },
      svg {
        class: "w-5 h-5 transition-transform duration-300 group-hover:rotate-90",
        view_box: "0 0 24 24",
        fill: "none",
        stroke: "currentColor",
        stroke_width: "2",
        stroke_linecap: "round",
        stroke_linejoin: "round",
        path { d: "M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z" }
        circle { cx: "12", cy: "12", r: "3" }
      }
    }
  }
}


#[component]
pub fn CloseButton() -> Element {
  let route = use_route::<Route>();
  rsx! {
    button {
      class: "absolute right-4 text-xl cursor-pointer hover:opacity-70",
      onclick: move |_| go_back(route.clone()),
      "\u{00d7}"
    }
  }
}
