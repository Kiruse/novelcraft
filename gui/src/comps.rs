use std::time::Duration;

use dioxus::prelude::*;
use novelcraft_engine::config::{
  DEFAULT_HOST, DEFAULT_MODEL, ModelConfig, ModelPurpose, NovelCraftConfig,
};

use crate::api;
use crate::auth::{self, AUTH};
use crate::error::GuiError;
use crate::events::QUILLS;
use crate::markdown::{self, Block, Segment};
use crate::screens::{Route, go_back};
use crate::toast::{Toast, push_toast};

/// Shared class list for text inputs.
pub const INPUT_CLASS: &str = "w-full min-w-0 px-3 py-2 bg-transparent border border-nc-border rounded-sm text-nc-text \
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
  /// Optional explanatory text, rendered as a `?` hint next to the label.
  #[props(default)]
  hint: Option<String>,
  children: Element,
}

/// Label above an input, spanning the full width.
#[component]
pub fn Field(props: FieldProps) -> Element {
  rsx! {
    div { class: "flex flex-col gap-1 w-full",
      div { class: "flex flex-row items-center gap-1.5",
        div { class: "text-nc-label", { props.label } }
        { props.hint.map(|hint| rsx! { Hint { text: hint } }) }
      }
      { props.children }
    }
  }
}

/// Small `?` in the highlight color with a hover tooltip. Pure CSS
/// (`group-hover`) — native `title` tooltips are unreliable in the
/// WebKit webviews the app renders in.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[allow(unused)]
pub enum HintAttachment {
  /// Tooltip opens below the marker. The default: above-the-marker
  /// tooltips clip at the tops of scroll containers (the Settings
  /// screen's first field).
  #[default]
  Bottom,
  /// Tooltip opens above the marker.
  Top,
  /// Tooltip opens to the left of the marker.
  Left,
  /// Tooltip opens to the right of the marker.
  Right,
}

#[derive(Props, Clone, PartialEq)]
pub struct HintProps {
  text: String,
  /// Which side of the marker the tooltip opens on.
  #[props(default)]
  attachment: HintAttachment,
}

#[component]
pub fn Hint(props: HintProps) -> Element {
  // Statically spelled-out class strings — the Tailwind scanner needs
  // the literals to compile the utilities.
  let position = match props.attachment {
    HintAttachment::Bottom => "top-full left-1/2 -translate-x-1/2 mt-1",
    HintAttachment::Top => "bottom-full left-1/2 -translate-x-1/2 mb-1",
    HintAttachment::Left => "right-full top-1/2 -translate-y-1/2 mr-1",
    HintAttachment::Right => "left-full top-1/2 -translate-y-1/2 ml-1",
  };

  rsx! {
    span { class: "relative group inline-flex items-center cursor-help",
      span { class: "text-nc-label text-xs font-bold rounded-full border border-nc-label/60 w-4 h-4 flex items-center justify-center shrink-0", "?" }
      span {
        class: "absolute hidden group-hover:block group-hover:z-50 {position} w-64 p-2 rounded-sm \
                bg-nc-bg border border-nc-border shadow-lg text-nc-text text-xs font-normal text-left normal-case",
        { props.text }
      }
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

/// Credential source of a model — the two [`ModelConfig`] variants:
/// requests flow through the NovelCraft hosted API authenticated with the
/// signed-in account, or to a user-configured OpenAI-compatible endpoint.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ModelSource {
  Login,
  Custom,
}

/// Editable fields of a single model config — shared by the Settings and
/// Onboarding screens. `source` picks the [`ModelConfig`] variant written
/// back: [`ModelSource::Login`] assembles its endpoint & credentials from
/// across the app (hosted inference host + the signed-in account's session
/// token), so only the model identifier is persisted; [`ModelSource::Custom`]
/// points at a manually configured OpenAI-compatible endpoint with its API
/// key.
#[derive(Debug, Clone, PartialEq)]
pub struct ModelFields {
  pub source: ModelSource,
  pub base_url: String,
  pub api_key: String,
  pub model: String,
}

impl Default for ModelFields {
  fn default() -> Self {
    Self {
      source: ModelSource::Login,
      base_url: DEFAULT_HOST.to_string(),
      api_key: String::new(),
      model: DEFAULT_MODEL.to_string(),
    }
  }
}

impl ModelFields {
  pub fn from_config(model_cfg: &ModelConfig) -> Self {
    match model_cfg {
      ModelConfig::Login { model } => Self {
        source: ModelSource::Login,
        base_url: DEFAULT_HOST.to_string(),
        api_key: String::new(),
        model: model.clone(),
      },
      ModelConfig::OpenAiCompatible { base_url, api_key, model } => Self {
        source: ModelSource::Custom,
        base_url: base_url.clone(),
        api_key: api_key.clone(),
        model: model.clone(),
      },
    }
  }

  /// Write the fields into the model config under `purpose`.
  pub fn write_to(self, purpose: ModelPurpose, config: &mut NovelCraftConfig) {
    let ModelFields { source, base_url, api_key, model } = self;
    config.models[purpose] = match source {
      ModelSource::Login => ModelConfig::Login {
        model: model.trim().to_string(),
      },
      ModelSource::Custom => ModelConfig::OpenAiCompatible {
        base_url: base_url.trim().to_string(),
        api_key: api_key.trim().to_string(),
        model: model.trim().to_string(),
      },
    };
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

/// Grace period after the last endpoint-relevant edit before the model
/// suggestions are refetched, so bursts of keystrokes coalesce into a
/// single request.
const MODELS_DEBOUNCE: Duration = Duration::from_millis(300);

/// Fetch state of a model suggestion list: in flight (`None`), failed, or
/// the sorted model identifiers of the configured endpoint.
type ModelsState = Option<Result<Vec<String>, String>>;

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
  let set_source = move |source| fields.write().source = source;

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

  let login = fields.read().source == ModelSource::Login;

  // Model suggestions of the endpoint this group's requests go to: the
  // hosted inference API for Login sources, the configured endpoint itself
  // for custom ones. Fetched on mount and whenever the refetch trigger
  // (see `endpoint`) settles, debounced, with stale responses discarded
  // via the generation counter.
  let mut models: Signal<ModelsState> = use_signal(|| None);
  let mut models_gen = use_signal(|| 0usize);
  let mut models_open = use_signal(|| false);

  // The refetch trigger: the credential source plus, for custom endpoints,
  // the target URL & key — exactly the fields that decide where the
  // suggestion request goes (typing the model name itself must not
  // refetch; the memo's equality gate skips those runs).
  let endpoint = use_memo(move || {
    let fields = fields.read();
    match fields.source {
      ModelSource::Login => (fields.source, String::new(), String::new()),
      ModelSource::Custom => (
        fields.source,
        fields.base_url.trim().to_string(),
        fields.api_key.trim().to_string(),
      ),
    }
  });

  use_effect(move || {
    let (source, base_url, api_key) = endpoint.read().clone();
    // Bump the generation counter through its write guard — reading it
    // here would subscribe this effect to its own writes and re-run it
    // forever.
    let generation = {
      let mut counter = models_gen.write();
      *counter += 1;
      *counter
    };
    spawn(async move {
      crate::time::sleep(MODELS_DEBOUNCE).await;
      if *models_gen.read() != generation {
        return;
      }
      let result = match source {
        ModelSource::Login => api::client().list_models().await,
        ModelSource::Custom if !base_url.is_empty() => {
          let key = (!api_key.is_empty()).then_some(api_key.as_str());
          api::list_models_at(&base_url, key).await
        }
        ModelSource::Custom => Err(GuiError::Api("No endpoint configured".to_string())),
      };
      if *models_gen.read() == generation {
        let result = result
          .map(|response| response.ids())
          .map_err(|err| err.to_string());
        models.set(Some(result));
      }
    });
  });

  rsx! {
    FieldGroup { label: props.purpose.as_str(),
      Field {
        label: "Source",
        hint: "Where the requests go: Login runs the model on NovelCraft's hosted inference, authenticated with the signed-in account. API Key sends requests to any OpenAI-compatible endpoint with a manually configured key.",
        SourceToggle { value: fields.read().source, on_change: set_source }
      }
      { if login { rsx! {} } else {
          rsx! {
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
          }
        } }
      Field { label: "Model",
        div { class: "flex flex-row items-center gap-1 w-full",
          div { class: "relative flex-1 min-w-0",
            input {
              class: INPUT_CLASS,
              value: "{fields.read().model}",
              oninput: set_model,
              onfocus: move |_| models_open.set(true),
              onclick: move |_| models_open.set(true),
              onkeydown: move |ev: KeyboardEvent| {
                if ev.key() == Key::Escape {
                  models_open.set(false);
                }
              },
            }
            { if *models_open.read() {
                rsx! {
                  // Click-away catcher behind the floating list.
                  div {
                    class: "fixed inset-0 z-40 cursor-default",
                    onclick: move |_| models_open.set(false),
                  }
                  ModelOptions {
                    state: models.read().clone(),
                    filter: fields.read().model.clone(),
                    on_select: move |id: String| {
                      fields.write().model = id;
                      models_open.set(false);
                    },
                  }
                }
              } else { rsx! {} } }
          }
          ApplyEverywhere { on_click: spread_model }
        }
      }
    }
  }
}

/// Floating suggestion list under the model input: the configured
/// endpoint's models, locally filtered by the current input value
/// (case-insensitive substring match; an empty input lists everything).
/// Purely assistive — picking an option overrides the input, but the
/// input stays a free-text field: custom model identifiers typed by hand
/// are kept as-is and the list never validates them.
#[derive(Props, Clone, PartialEq)]
struct ModelOptionsProps {
  state: ModelsState,
  filter: String,
  on_select: EventHandler<String>,
}

#[component]
fn ModelOptions(props: ModelOptionsProps) -> Element {
  let filter = props.filter.trim().to_lowercase();
  rsx! {
    div { class: "absolute left-0 right-0 top-full mt-1 z-50 flex flex-col max-h-64 overflow-y-auto \
                  rounded-sm border border-nc-border bg-nc-bg shadow-lg",
      { match &props.state {
          None => rsx! {
            div { class: "px-3 py-1.5", Loading { text: "Loading models ..." } }
          },
          Some(Err(_)) => rsx! {
            div { class: "px-3 py-1.5 text-sm text-nc-label", "Failed to load models" }
          },
          Some(Ok(list)) => {
            let matches: Vec<String> = list
              .iter()
              .filter(|id| id.to_lowercase().contains(&filter))
              .cloned()
              .collect();
            if matches.is_empty() {
              rsx! {
                div { class: "px-3 py-1.5 text-sm text-nc-label", "No matching models" }
              }
            } else {
              rsx! {
                for id in matches {
                  button {
                    key: "{id}",
                    class: "px-3 py-1.5 text-sm text-left cursor-pointer hover:bg-nc-text/10 truncate",
                    onclick: move |_| props.on_select.call(id.clone()),
                    { id.clone() }
                  }
                }
              }
            }
          }
      } }
    }
  }
}

/// Options of the [`SourceToggle`]: credential sources with their labels.
const MODEL_SOURCES: [(ModelSource, &str); 2] = [
  (ModelSource::Login, "Login"),
  (ModelSource::Custom, "API Key"),
];

/// Conjoined radio buttons for the model credential source — exactly one
/// of the two segments is active at a time.
#[derive(Props, Clone, PartialEq)]
pub struct SourceToggleProps {
  value: ModelSource,
  on_change: EventHandler<ModelSource>,
}

/// Segment styling of the [`SourceToggle`]: state classes (unselected /
/// selected), corner rounding and shared-border trims. Statically
/// spelled-out literals — the Tailwind scanner compiles the utilities.
const TOGGLE_STATE: [&str; 2] = [
  "bg-transparent text-nc-text border-nc-border hover:bg-nc-text/10 hover:border-nc-label",
  "bg-nc-text text-nc-bg border-nc-text hover:bg-nc-text/85",
];
const TOGGLE_CORNERS: [&str; 2] = ["rounded-l-sm rounded-r-none", "rounded-r-sm rounded-l-none"];
const TOGGLE_EDGES: [&str; 2] = ["border-r-0", ""];

#[component]
pub fn SourceToggle(props: SourceToggleProps) -> Element {
  rsx! {
    div { class: "flex w-full",
      for (idx, (source, label)) in MODEL_SOURCES.iter().enumerate() {
        button {
          key: "{idx}",
          class: "flex-1 py-1.5 text-sm border cursor-pointer \
                  {TOGGLE_STATE[(*source == props.value) as usize]} \
                  {TOGGLE_CORNERS[idx]} {TOGGLE_EDGES[idx]}",
          onclick: move |_| props.on_change.call(*source),
          { *label }
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

/// Standard screen layout: centered top bar with title + (back arrow left
/// of the title, gear, or nothing) and the UserBox at the rightmost
/// position, above a centered, scrollable content column.
#[derive(Props, Clone, PartialEq)]
pub struct ScreenProps {
  title: String,
  /// Hide the title-bar controls entirely — for screens that must not be
  /// exited (onboarding).
  #[props(default)]
  plain: bool,
  #[props(default)]
  navback: bool,
  /// Hide the UserBox (e.g. on the login screen itself).
  #[props(default)]
  nouserbox: bool,
  children: Element,
}

#[component]
pub fn Screen(props: ScreenProps) -> Element {
  rsx! {
    div { class: "flex flex-col items-center w-full h-full",
      div { class: "relative w-full flex flex-row justify-center items-center",
        div { class: "relative",
          { if props.navback { rsx! { NavBack {} } } else { rsx! {} } }
          h1 { class: "text-3xl", { props.title } }
        }
        { if props.plain {
            rsx! {}
          } else {
            rsx! {
              div { class: "absolute right-4 flex flex-row items-center gap-1",
                { if props.navback { rsx! {} } else { rsx! { SettingsGear {} } } }
                { if props.nouserbox { rsx! {} } else { rsx! { QuillsBox {} UserBox {} } } }
              }
            }
          } }
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
      class: "group p-1 cursor-pointer text-nc-text hover:opacity-70",
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

/// Back arrow rendered to the immediate left of the screen title — walks
/// the router history one step back.
#[component]
pub fn NavBack() -> Element {
  let route = use_route::<Route>();
  rsx! {
    button {
      class: "absolute right-full h-full mr-2 p-1 cursor-pointer text-nc-text hover:opacity-70",
      onclick: move |_| go_back(route.clone()),
      svg {
        class: "w-7 h-5",
        view_box: "0 0 24 24",
        fill: "none",
        stroke: "currentColor",
        stroke_width: "2",
        stroke_linecap: "round",
        stroke_linejoin: "round",
        path { d: "M27 12H5" }
        path { d: "M12 19l-7-7 7-7" }
      }
    }
  }
}

/// Quill balance next to the title bar's `UserBox`: the live balance from
/// the shared account-event stream ([`crate::events::QUILLS`], rendered as
/// an em dash while the first seed is in flight). Clicking claims the
/// daily free quills — a success toast on success, an informational toast
/// when the daily claim was already used (HTTP 409).
#[component]
pub fn QuillsBox() -> Element {
  let signed_in = AUTH.read().user.is_some();
  let mut claiming = use_signal(|| false);

  let claim = move |_| {
    claiming.set(true);
    spawn(async move {
      match api::client().claim_quills().await {
        Ok(balance) => {
          *QUILLS.write() = Some(balance.quills);
          push_toast(Toast::success("Claimed the daily quills"));
        }
        Err(GuiError::ApiStatus { status: 409, .. }) => {
          push_toast(Toast::info("Daily quills already claimed today"));
        }
        Err(err) => push_toast(Toast::error(format!(
          "Failed to claim the daily quills: {err}"
        ))),
      }
      claiming.set(false);
    });
  };

  if !signed_in {
    return rsx! {};
  }

  rsx! {
    button {
      class: "px-2 py-1 text-sm rounded-sm border border-nc-border cursor-pointer \
              hover:border-nc-label disabled:opacity-50 disabled:cursor-default",
      title: "Claim the daily free quills",
      disabled: *claiming.read(),
      onclick: claim,
      { match *QUILLS.read() {
          Some(quills) => rsx! { "{quills:.2} quills" },
          None => rsx! { "— quills" },
      } }
    }
  }
}

/// Account control at the rightmost of the title bar: a plain login button
/// when signed out, or the username (falling back to the email) opening a
/// popover with account options — logout, for now.
#[component]
pub fn UserBox() -> Element {
  let mut open = use_signal(|| false);
  let nav = navigator();
  let state = AUTH.read().clone();

  let logout = move |_| {
    spawn(async move {
      auth::logout().await;
    });
  };

  rsx! {
    div { class: "relative",
      { if let Some(user) = state.user {
          let label = user.username.clone().unwrap_or_else(|| user.email.clone());
          rsx! {
            button {
              class: "max-w-40 px-2 py-1 text-sm rounded-sm border border-nc-border cursor-pointer hover:border-nc-label truncate",
              onclick: move |_| open.toggle(),
              { label.clone() }
            }
            { if *open.read() {
                rsx! {
                  // Click-away catcher behind the popover.
                  div {
                    class: "fixed inset-0 z-40 cursor-default",
                    onclick: move |_| open.set(false),
                  }
                  div { class: "absolute right-0 top-full mt-1 z-50 flex flex-col gap-2 w-56 p-3 rounded-sm border border-nc-border bg-nc-bg shadow-lg",
                    div { class: "flex flex-col w-full",
                      div { class: "truncate", { label.clone() } }
                      div { class: "text-sm text-nc-label truncate", { user.email.clone() } }
                    }
                    Button { label: "Log out", variant: BtnVariant::Secondary, on_click: logout }
                  }
                }
              } else { rsx! {} } }
          }
        } else {
          rsx! {
            button {
              class: "px-2 py-1 text-sm rounded-sm border border-nc-border cursor-pointer hover:border-nc-label disabled:opacity-50",
              disabled: state.restoring,
              onclick: move |_| {
                nav.push(Route::Login {});
              },
              "Log in"
            }
          }
        } }
    }
  }
}
