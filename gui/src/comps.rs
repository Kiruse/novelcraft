use dioxus::prelude::*;
use novelcraft_engine::config::{DEFAULT_HOST, DEFAULT_MODEL, ModelConfig, ModelPurpose};

use crate::markdown::{self, Block, Segment};
use crate::screens::{go_back, Route};

/// Shared class list for text inputs.
pub const INPUT_CLASS: &str =
  "w-full min-w-0 px-3 py-2 bg-transparent border border-nc-border rounded-sm text-nc-text \
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

/// Editable fields of a single model config — shared by the Settings and
/// Onboarding screens.
#[derive(Debug, Clone, PartialEq)]
pub struct ModelFields {
  pub base_url: String,
  pub api_key: String,
  pub model: String,
}

impl Default for ModelFields {
  fn default() -> Self {
    Self {
      base_url: DEFAULT_HOST.to_string(),
      api_key: String::new(),
      model: DEFAULT_MODEL.to_string(),
    }
  }
}

impl ModelFields {
  pub fn from_config(config: &ModelConfig) -> Self {
    let ModelConfig::OpenAi {
      base_url,
      api_key,
      model,
    } = config;
    Self {
      base_url: base_url.clone(),
      api_key: api_key.clone(),
      model: model.clone(),
    }
  }

  pub fn into_config(self) -> ModelConfig {
    ModelConfig::OpenAi {
      base_url: self.base_url.trim().to_string(),
      api_key: self.api_key.clone(),
      model: self.model.trim().to_string(),
    }
  }
}

/// Base URL / API key / model fields for one [`ModelPurpose`], with an
/// "apply everywhere" adornment on each field that copies the value to the
/// corresponding fields of the other two model usages.
#[derive(Props, Clone, PartialEq)]
pub struct ModelGroupProps {
  purpose: ModelPurpose,
  /// Editable fields of all three model usages, indexed by
  /// [`purpose_index`] — the entry for `purpose` is edited in place, the
  /// other two receive spread values.
  models: [Signal<ModelFields>; 3],
}

/// Index into a `[Signal<ModelFields>; 3]` (see [`ModelGroupProps::models`]).
fn purpose_index(purpose: ModelPurpose) -> usize {
  match purpose {
    ModelPurpose::DungeonMaster => 0,
    ModelPurpose::Suggestions => 1,
    ModelPurpose::Summarizer => 2,
  }
}

#[component]
pub fn ModelGroup(props: ModelGroupProps) -> Element {
  let mut fields = props.models[purpose_index(props.purpose)];

  let set_base_url = move |ev: FormEvent| {
    fields.write().base_url = ev.value();
  };
  let set_api_key = move |ev: FormEvent| {
    fields.write().api_key = ev.value();
  };
  let set_model = move |ev: FormEvent| {
    fields.write().model = ev.value();
  };

  let spread_base_url = move |_| {
    let value = fields.read().base_url.clone();
    for mut signal in props.models {
      signal.write().base_url = value.clone();
    }
  };
  let spread_api_key = move |_| {
    let value = fields.read().api_key.clone();
    for mut signal in props.models {
      signal.write().api_key = value.clone();
    }
  };
  let spread_model = move |_| {
    let value = fields.read().model.clone();
    for mut signal in props.models {
      signal.write().model = value.clone();
    }
  };

  rsx! {
    FieldGroup { label: props.purpose.as_str(),
      Field { label: "Base URL",
        div { class: "flex flex-row items-center gap-1 w-full",
          input {
            class: INPUT_CLASS,
            placeholder: DEFAULT_HOST,
            value: "{fields.read().base_url}",
            oninput: set_base_url,
          }
          ApplyEverywhere { on_click: spread_base_url }
        }
      }
      Field { label: "API Key",
        div { class: "flex flex-row items-center gap-1 w-full",
          input {
            class: INPUT_CLASS,
            r#type: "password",
            value: "{fields.read().api_key}",
            oninput: set_api_key,
          }
          ApplyEverywhere { on_click: spread_api_key }
        }
      }
      Field { label: "Model",
        div { class: "flex flex-row items-center gap-1 w-full",
          input {
            class: INPUT_CLASS,
            value: "{fields.read().model}",
            oninput: set_model,
          }
          ApplyEverywhere { on_click: spread_model }
        }
      }
    }
  }
}

/// Small icon button appended to a model field — copies the field's value
/// to the corresponding fields of the other model usages.
#[derive(Props, Clone, PartialEq)]
struct ApplyEverywhereProps {
  on_click: EventHandler,
}

#[component]
fn ApplyEverywhere(props: ApplyEverywhereProps) -> Element {
  rsx! {
    button {
      class: "shrink-0 p-1.5 rounded-sm text-nc-text/50 hover:text-nc-text hover:bg-nc-text/10 cursor-pointer",
      title: "Copy this value to the other model usages",
      onclick: move |_| props.on_click.call(()),
      svg {
        class: "w-4 h-4",
        view_box: "0 0 24 24",
        fill: "none",
        stroke: "currentColor",
        stroke_width: "2",
        stroke_linecap: "round",
        stroke_linejoin: "round",
        rect { x: "9", y: "9", width: "13", height: "13", rx: "2", ry: "2" }
        path { d: "M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" }
      }
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
          Block::List { ordered, items } => rsx! {
            { if *ordered {
              rsx! {
                ol { key: "{idx}", for (i, item) in items.iter().enumerate() {
                  li { key: "{i}", { render_segments(item) } }
                } }
              }
            } else {
              rsx! {
                ul { key: "{idx}", for (i, item) in items.iter().enumerate() {
                  li { key: "{i}", { render_segments(item) } }
                } }
              }
            } }
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

/// Standard screen layout: centered top bar with title + (gear, close, or
/// nothing), above a centered, scrollable content column.
#[derive(Props, Clone, PartialEq)]
pub struct ScreenProps {
  title: String,
  /// Hide the corner control entirely — for screens that must not be
  /// exited (onboarding).
  #[props(default)]
  plain: bool,
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
        { if props.plain {
            rsx! {}
          } else if props.closable {
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
