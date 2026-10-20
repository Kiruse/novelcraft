use std::collections::HashMap;

use dioxus::prelude::*;
use novelcraft_engine::game::profile::ProfileV1;
use novelcraft_engine::game::turns::TurnV2;

use crate::comps::{Button, BtnVariant, CloseButton, Field, Loading, Markdown, INPUT_CLASS};
use crate::engine;
use crate::modules::{module_context, EditingLeaf, StatePath};
use crate::state::{StreamState, PREMISE_PAGE, PROFILES, REASONING, SESSION, STREAM, TURN, TURN_INDEX};

/// Swipe distance (in logical pixels) required to flip to the adjacent page.
const SWIPE_THRESHOLD: f64 = 60.0;
/// How far the page can be dragged sideways during a swipe gesture.
const DRAG_LIMIT: f64 = 160.0;
/// Width of the toggleable story memory side bar.
const SIDEBAR_WIDTH_PX: f64 = 280.0;
/// Classes of the translucent boxes holding story text (premise, prompts,
/// responses): one centered column at most 600px wide, so background images
/// stay visible around it.
const TEXT_CARD: &str =
  "w-full max-w-[600px] mx-auto p-3 rounded-md border border-black/90 bg-black/70";
/// User prompt bubbles get a light tint for visual distinction.
const PROMPT_CARD: &str =
  "w-full max-w-[600px] mx-auto p-3 rounded-md border border-black/90 bg-white/25";

#[component]
pub fn StoryGameplay(id: String) -> Element {
  let mut input = use_signal(String::new);
  let popover_open = use_signal(|| false);
  let mut sidebar_open = use_signal(|| false);
  // Profile currently being edited, if the dialog is open.
  let mut editing_profile: Signal<Option<ProfileV1>> = use_signal(|| None);
  // Horizontal start position of the active swipe gesture, if any.
  let mut swipe_start: Signal<Option<f64>> = use_signal(|| None);
  // Current horizontal drag offset of the page content.
  let mut drag_dx: Signal<f64> = use_signal(|| 0.0);
  let mut dragging: Signal<bool> = use_signal(|| false);
  // Text-selection mode (toggled via the caret button under the last
  // response): swiping is disabled until it is switched off again, so text
  // can be selected & copied.
  let select_mode = use_signal(|| false);

  // Load the session into the engine, then open its last turn.
  use_hook(move || {
    let id = id.clone();
    spawn(async move {
      engine::play_session(id).await;
    });
  });

  let title_text = SESSION
    .read()
    .value()
    .map(|session| session.title.clone())
    .unwrap_or_else(|| "...".to_string());

  let streaming = STREAM.read().is_some();
  let can_send = !streaming
    && engine::session_turn_count().is_some()
    && !input.read().trim().is_empty();

  let mut submit = move || {
    if STREAM.read().is_some() {
      return;
    }
    let content = input.read().trim().to_string();
    if content.is_empty() {
      return;
    }
    input.set(String::new());

    // Prompting from the premise page starts the story over from scratch.
    if *TURN_INDEX.read() == PREMISE_PAGE {
      engine::start_over(content);
      return;
    }

    let Some(count) = engine::session_turn_count() else { return };
    let turn_idx = *TURN_INDEX.read();
    // Fork from the current turn only when not already on the last turn.
    let from_turn = ((turn_idx as usize) + 1 < count).then_some(turn_idx as usize);
    engine::prompt(content, from_turn);
  };

  rsx! {
    div {
      class: "flex flex-col items-center w-full h-full bg-nc-bg text-nc-text",
      onkeydown: move |ev: KeyboardEvent| {
        // Turn navigation via alt+arrows, mirroring the previous global
        // keybindings (scoped to the gameplay screen).
        if ev.data.modifiers().contains(Modifiers::ALT) {
          match ev.data.key() {
            Key::ArrowLeft => {
              ev.prevent_default();
              engine::navigate(-1);
            }
            Key::ArrowRight => {
              ev.prevent_default();
              engine::navigate(1);
            }
            _ => {}
          }
        }
      },

      div { class: "relative w-full flex flex-row justify-center p-2",
        button {
          class: "absolute right-14 text-xl cursor-pointer hover:opacity-70",
          onclick: move |_| sidebar_open.toggle(),
          "\u{2630}"
        }
        div { class: "text-3xl", { title_text } }
        CloseButton {}
      }

      div { class: "flex flex-row flex-1 min-h-0 w-full",
        div {
          class: if *select_mode.read() {
            "flex-1 min-w-0 h-full flex flex-col gap-3 p-4 overflow-y-auto"
          } else {
            "flex-1 min-w-0 h-full flex flex-col gap-3 p-4 overflow-y-auto select-none"
          },
          onpointerdown: move |ev: PointerEvent| {
            // While selection mode is active, swiping is disabled so text
            // can be selected & copied.
            if *select_mode.read() {
              return;
            }
            swipe_start.set(Some(ev.data.client_coordinates().x));
            dragging.set(true);
          },
          onpointermove: move |ev: PointerEvent| {
            let Some(start) = *swipe_start.read() else { return };
            let dx = ev.data.client_coordinates().x - start;
            drag_dx.set(dx.clamp(-DRAG_LIMIT, DRAG_LIMIT));
          },
          onpointerup: move |_| {
            if swipe_start.read().is_none() {
              return;
            }
            swipe_start.set(None);
            dragging.set(false);
            let dx = *drag_dx.read();
            drag_dx.set(0.0);

            let Some(count) = engine::session_turn_count() else { return };
            let current = *TURN_INDEX.read();
            // Flip only on release, and only when a page exists that way.
            if dx <= -SWIPE_THRESHOLD && current < count as isize - 1 {
              engine::navigate(1);
            } else if dx >= SWIPE_THRESHOLD && current > PREMISE_PAGE {
              engine::navigate(-1);
            }
          },
          onpointercancel: move |_| {
            swipe_start.set(None);
            dragging.set(false);
            drag_dx.set(0.0);
          },
          onpointerleave: move |_| {
            swipe_start.set(None);
            dragging.set(false);
            drag_dx.set(0.0);
          },
          div {
            class: if *dragging.read() {
              "w-full"
            } else {
              "w-full transition-transform duration-150 ease-out"
            },
            style: "transform: translateX({*drag_dx.read()}px)",
            { turn_content_ui(select_mode) }
          }
        }
        { sidebar_open.read().then(|| rsx! { Sidebar {} }) }
      }

      { status_row_ui() }

      div { class: "w-full flex flex-row items-end gap-2 p-4",
        { avatar_ui(popover_open, editing_profile) }
        div { class: "flex-1 min-w-0",
          textarea {
            class: INPUT_CLASS,
            rows: 2,
            placeholder: "What do you do or say?",
            value: "{input}",
            oninput: move |ev: FormEvent| input.set(ev.value()),
            onkeydown: move |ev: KeyboardEvent| {
              if ev.data.key() == Key::Enter && !ev.data.modifiers().contains(Modifiers::SHIFT) {
                ev.prevent_default();
                submit();
              }
            },
          }
        }
        button {
          class: if can_send {
            "flex-none px-4 py-2 rounded-sm cursor-pointer bg-nc-text text-nc-bg hover:bg-nc-text/85"
          } else {
            "flex-none px-4 py-2 rounded-sm bg-nc-text/10 text-nc-text/90"
          },
          disabled: !can_send,
          onclick: move |_| submit(),
          "Send"
        }
      }

      { editing_profile.read().as_ref().map(|profile| rsx! {
          ProfileDialog {
            key: "{profile.id}",
            profile: profile.clone(),
            on_close: move |_| editing_profile.set(None),
            on_save: move |updated: ProfileV1| {
              editing_profile.set(None);
              spawn(async move {
                let _ = engine::update_profile_async(updated).await;
              });
            },
          }
      }) }
    }
  }
}

fn turn_content_ui(select_mode: Signal<bool>) -> Element {
  if let Some(stream) = &*STREAM.read() {
    return stream_card(stream);
  }

  let session_q = SESSION.read();
  let Some(session) = session_q.value() else {
    return if session_q.error().is_some() {
      rsx! { div { class: "text-nc-dangerfg", "Failed to load session" } }
    } else {
      rsx! { Loading { text: "Loading session ..." } }
    };
  };

  // Fresh session — there are no turns to fetch yet.
  if session.turn_count() == 0 || *TURN_INDEX.read() == PREMISE_PAGE {
    return rsx! {
      div { class: "w-full flex flex-col gap-3",
        { premise_card(&session.exposition) }
        div { class: "mx-auto", { selection_toggle(select_mode) } }
      }
    };
  }

  let turn_q = TURN.read();
  let Some(turn) = turn_q.value() else {
    return if turn_q.error().is_some() {
      rsx! { div { class: "text-nc-dangerfg", "Failed to load turn" } }
    } else {
      rsx! { Loading { text: "Loading turn ..." } }
    };
  };

  if turn_is_empty(turn) {
    rsx! {
      div { class: "w-full flex flex-col gap-3",
        { premise_card(&session.exposition) }
        div { class: "mx-auto", { selection_toggle(select_mode) } }
      }
    }
  } else {
    let is_last = *TURN_INDEX.read() as usize + 1 == session.turn_count();
    turn_card(turn, is_last, select_mode)
  }
}

/// The story's opening view: the premise, until the first real turn exists.
fn premise_card(exposition: &str) -> Element {
  if exposition.trim().is_empty() {
    return rsx! { "Send your first prompt to begin the story ..." };
  }
  rsx! {
    div { class: TEXT_CARD,
      Markdown { text: exposition.to_string() }
    }
  }
}

fn status_row_ui() -> Element {
  let count = engine::session_turn_count();
  let turn_index = *TURN_INDEX.read();
  let streaming = STREAM.read().is_some();
  let show_indicator = !streaming && count.is_some_and(|count| count > 0);
  let show_jump = !streaming
    && count.is_some_and(|count| count > 0 && turn_index + 1 < count as isize);

  let page_label = if turn_index == PREMISE_PAGE {
    "Premise".to_string()
  } else {
    format!("{}/{}", turn_index + 1, count.unwrap_or(0))
  };

  rsx! {
    div { class: "w-full flex flex-row justify-center items-center gap-3",
      { show_indicator.then(|| {
          rsx! {
            div { class: "text-sm text-nc-label", { page_label } }
          }
      }) }
      { show_jump.then(|| {
          rsx! {
            button {
              class: "px-3 py-1 text-sm border rounded-full border-nc-border cursor-pointer hover:border-nc-label hover:bg-nc-text/10",
              onclick: move |_| engine::jump_to_end(),
              "Jump to end \u{2193}"
            }
          }
      }) }
    }
  }
}

/// The profile ID the gameplay session effectively runs with: the
/// session's stored profile, falling back to the global one.
fn active_profile_id() -> Option<String> {
  let session_profile = SESSION.read().value().and_then(|s| s.profile.clone());
  session_profile.or_else(|| {
    let profiles = PROFILES.read();
    profiles.value().and_then(|p| p.active_profile.clone())
  })
}

fn avatar_ui(
  mut popover_open: Signal<bool>,
  editing_profile: Signal<Option<ProfileV1>>,
) -> Element {
  let profiles_q = PROFILES.read();
  let initial = match (active_profile_id(), profiles_q.value()) {
    (Some(id), Some(profiles)) => profiles
      .profiles
      .iter()
      .find(|profile| profile.id == id)
      .and_then(|profile| profile.name.chars().next())
      .map(|ch| ch.to_string())
      .unwrap_or_else(|| "?".to_string()),
    _ => "?".to_string(),
  };

  rsx! {
    div { class: "relative flex-none",
      button {
        class: "w-10 h-10 rounded-full flex items-center justify-center text-lg bg-nc-text text-nc-bg cursor-pointer hover:opacity-85",
        onclick: move |_| popover_open.toggle(),
        { initial }
      }
      { popover_open.read().then(|| rsx! {
          // Clickaway backdrop — any click outside the popover closes it.
          div {
            class: "fixed inset-0 z-40 cursor-default",
            onclick: move |_| popover_open.set(false),
          }
          { profile_popover_ui(popover_open, editing_profile) }
      }) }
    }
  }
}

fn profile_popover_ui(
  mut popover_open: Signal<bool>,
  mut editing_profile: Signal<Option<ProfileV1>>,
) -> Element {
  let profiles_q = PROFILES.read();
  let active_id = active_profile_id();

  rsx! {
    div {
      class: "absolute bottom-full left-0 z-50 mb-2 w-56 max-h-70 overflow-y-auto flex flex-col gap-1 p-2 bg-nc-bg border rounded-sm border-nc-border shadow-[0_4px_12px_rgba(0,0,0,0.4)]",
      { match profiles_q.value() {
          Some(profiles) => rsx! {
            for profile in profiles.profiles.iter() {
              {
                let is_active = active_id.as_deref() == Some(profile.id.as_str());
                let pid = profile.id.clone();
                let editable = profile.clone();
                rsx! {
                  div {
                    key: "{profile.id}",
                    class: "rounded-sm flex flex-row items-center hover:bg-nc-text/10",
                    button {
                      class: "flex-1 min-w-0 px-2 py-1 flex flex-row justify-between items-center gap-2 text-left cursor-pointer",
                      onclick: move |_| {
                        popover_open.set(false);
                        let pid = pid.clone();
                        spawn(async move {
                          engine::switch_profile(pid).await;
                        });
                      },
                      span { class: "truncate", { profile.name.clone() } }
                      { is_active.then(|| rsx! { span { class: "text-nc-success", "\u{2713}" } }) }
                    }
                    button {
                      class: "flex-none p-1 mr-1 flex items-center cursor-pointer text-nc-text/60 hover:text-nc-text",
                      title: "Edit profile",
                      onclick: move |_| {
                        popover_open.set(false);
                        editing_profile.set(Some(editable.clone()));
                      },
                      svg {
                        class: "w-3.5 h-3.5",
                        view_box: "0 0 24 24",
                        fill: "none",
                        stroke: "currentColor",
                        stroke_width: "2",
                        stroke_linecap: "round",
                        stroke_linejoin: "round",
                        path { d: "M17 3a2.85 2.83 0 1 1 4 4L7.5 20.5 2 22l1.5-5.5Z" }
                        path { d: "m15 5 4 4" }
                      }
                    }
                  }
                }
              }
            }
          },
          None => rsx! {},
      } }
      button {
        class: "px-2 py-1 rounded-sm flex flex-row items-center gap-2 cursor-pointer text-nc-label hover:bg-nc-text/10",
        onclick: move |_| {
          popover_open.set(false);
          spawn(async move {
            if let Ok(profile) = engine::create_profile_async().await {
              editing_profile.set(Some(profile));
            }
          });
        },
        span { class: "text-base leading-none", "+" }
        span { "Create new" }
      }
    }
  }
}

#[component]
fn Sidebar() -> Element {
  let mut prompt_text: Signal<Option<Result<String, String>>> = use_signal(|| None);
  let mut module_states: Signal<HashMap<String, Result<serde_json::Value, String>>> =
    use_signal(HashMap::new);
  // Leaf currently being edited (path + draft) across all module sections.
  let editing: Signal<EditingLeaf> = use_signal(|| None);

  // Fetch the effective prompt and the active modules' states — on open, on
  // session change, and whenever the viewed turn changes (navigation or a
  // completed generation mutating the turn's content).
  use_effect(move || {
    let ids = SESSION.read().value()
      .map(|session| {
        let mut ids: Vec<String> = session.modules().cloned().collect();
        ids.sort();
        ids
      })
      .unwrap_or_default();
    let _ = TURN.read().value();
    let _ = *TURN_INDEX.read();
    spawn(async move {
      let res = engine::effective_prompt_async().await.map_err(|err| err.to_string());
      prompt_text.set(Some(res));
      for id in ids {
        let res = engine::module_state_async(id.clone()).await.map_err(|err| err.to_string());
        module_states.write().insert(id, res);
      }
    });
  });

  let premise = SESSION.read().value()
    .map(|session| session.exposition.clone())
    .unwrap_or_default();

  // The sidebar only edits on the last page: the gamestate (and thus the
  // rendered module states) reflect ALL turns' tool calls, so older pages
  // don't line up with what an edit would persist.
  let editable = {
    let count = SESSION.read().value().map(|session| session.turn_count()).unwrap_or_default();
    *TURN_INDEX.read() + 1 == count as isize
  };

  // Tool calls of the turn currently being viewed.
  let mut tool_calls: Vec<(String, String, Option<String>)> = Vec::new();
  if let Some(turn) = TURN.read().value() {
    for response in &turn.responses {
      for tc in &response.tool_calls {
        let result = response
          .tool_responses
          .iter()
          .find(|(id, _)| id == &tc.id)
          .map(|(_, content)| content.clone());
        let mut args = serde_json::from_str::<serde_json::Value>(&tc.arguments)
          .map(|value| value.to_string())
          .unwrap_or_else(|_| tc.arguments.clone());
        if args.chars().count() > 120 {
          args = format!("{}…", args.chars().take(120).collect::<String>());
        }
        tool_calls.push((tc.name.clone(), args, result));
      }
    }
  }

  rsx! {
    div {
      class: "flex-none h-full border-l p-4 border-nc-border overflow-y-auto flex flex-col gap-3",
      style: "width: {SIDEBAR_WIDTH_PX}px",
      { (!premise.trim().is_empty()).then(|| rsx! {
          CollapsibleSection { title: "Premise", default_open: false,
            Markdown { text: premise.clone(), class: "h-100 overflow-y-auto" }
          }
      }) }
      { match prompt_text.read().as_ref() {
          None => rsx! {},
          Some(Err(err)) => rsx! { div { class: "text-nc-dangerfg text-sm", "{err}" } },
          Some(Ok(text)) => rsx! {
            CollapsibleSection { title: "Effective Prompt", default_open: false,
              div { class: "h-100 overflow-y-auto whitespace-pre-wrap text-sm", { text.clone() } }
            }
          },
      } }
      for id in session_module_ids() {
        {
          let ctx = module_context(&id);
          let state_res = module_states.read().get(&id).cloned();
          let title = ctx.title();
          let default_open = ctx.default_open();
          rsx! {
            { match state_res {
                None => rsx! {
                  CollapsibleSection { key: "{id}", title, default_open: false,
                    Loading { text: "Loading ..." }
                  }
                },
                Some(Err(err)) => rsx! {
                  CollapsibleSection { key: "{id}", title, default_open: true,
                    div { class: "text-nc-dangerfg text-sm", "{err}" }
                  }
                },
                Some(Ok(state)) if !ctx.has_content(&state) => rsx! {},
                Some(Ok(state)) => {
                  let commit_id = id.clone();
                  let commit_state = state.clone();
                  let on_commit = EventHandler::new(move |(path, value): (StatePath, String)| {
                    let ctx = module_context(&commit_id);
                    let new_state = ctx.edit(&commit_state, &path, value.clone());
                    module_states.write().insert(commit_id.clone(), Ok(new_state.clone()));
                    let mut keypath = vec![commit_id.clone()];
                    keypath.extend(path);
                    spawn(async move {
                      let _ = engine::edit_module_state_async(keypath, value).await;
                    });
                  });
                  rsx! {
                    CollapsibleSection { key: "{id}", title, default_open,
                      { ctx.render(&state, editable, editing, on_commit) }
                    }
                  }
                }
            } }
          }
        }
      }
      CollapsibleSection { title: "Reasoning", default_open: false,
        div { class: "h-100 overflow-y-auto text-sm whitespace-pre-wrap break-words",
          { let reasoning = REASONING.read().clone();
            if reasoning.trim().is_empty() {
              "Agent used no reasoning.".to_string()
            } else {
              reasoning
            } }
        }
      }
      CollapsibleSection { title: "Tool Calls", default_open: true,
        div { class: "flex flex-col gap-2 text-sm",
          { if tool_calls.is_empty() {
              rsx! { div { class: "text-nc-label", "None." } }
            } else {
              rsx! {
                for (idx, (name, args, result)) in tool_calls.into_iter().enumerate() {
                  div {
                    key: "{idx}",
                    class: "flex flex-col gap-0.5",
                    div { class: "text-nc-label break-words", { name } }
                    div { class: "text-xs text-nc-text/60 break-all", { args } }
                    { result.map(|result| rsx! {
                        div { class: "text-xs text-nc-text/80 break-words", { result } }
                    }) }
                  }
                }
              }
            } }
        }
      }
    }
  }
}

/// IDs of the active session's gameplay modules, sorted for stable order.
fn session_module_ids() -> Vec<String> {
  let mut ids: Vec<String> = SESSION
    .read()
    .value()
    .map(|session| session.modules().cloned().collect())
    .unwrap_or_default();
  ids.sort();
  ids
}

/// Collapsible sidebar section: a clickable heading toggling its children.
#[derive(Props, Clone, PartialEq)]
struct CollapsibleSectionProps {
  title: String,
  #[props(default)]
  default_open: bool,
  children: Element,
}

#[component]
fn CollapsibleSection(props: CollapsibleSectionProps) -> Element {
  let CollapsibleSectionProps { title, default_open, children } = props;
  let mut open = use_signal(|| default_open);

  rsx! {
    div { class: "flex flex-col gap-1",
      button {
        class: "flex flex-row items-center gap-1 w-full text-sm text-nc-label text-left cursor-pointer hover:opacity-80",
        onclick: move |_| open.toggle(),
        span { class: "w-3 flex-none", { if *open.read() { "\u{25be}" } else { "\u{25b8}" } } }
        span { { title } }
      }
      { open.read().then(move || rsx! {
          div { class: "flex flex-col gap-2 pl-4", { children } }
      }) }
    }
  }
}

/// The centered caret button toggling text-selection mode (shown on every
/// page, unlike the regenerate button).
fn selection_toggle(mut select_mode: Signal<bool>) -> Element {
  rsx! {
    button {
      class: if *select_mode.read() {
        "w-7 h-7 flex items-center justify-center rounded-full cursor-pointer text-nc-text"
      } else {
        "w-7 h-7 flex items-center justify-center rounded-full cursor-pointer text-nc-text/50 hover:text-nc-text hover:bg-nc-text/10"
      },
      title: "Toggle text selection",
      onclick: move |_| {
        // Leaving selection mode drops any leftover highlight.
        if *select_mode.read() {
          dioxus::document::eval("window.getSelection().removeAllRanges()");
        }
        select_mode.toggle();
      },
      svg {
        class: "w-4 h-4",
        view_box: "0 0 24 24",
        fill: "none",
        stroke: "currentColor",
        stroke_width: "2",
        stroke_linecap: "round",
        stroke_linejoin: "round",
        path { d: "M17 22h-1a1 1 0 0 1-1-1V3a1 1 0 0 1 1-1h1" }
        path { d: "M7 22h1a1 1 0 0 0 1-1V3a1 1 0 0 0-1-1H7" }
      }
    }
  }
}

fn turn_is_empty(turn: &TurnV2) -> bool {
  turn.prompt.is_none()
    && turn.responses.iter().all(|response| response.content.is_none())
}

fn turn_card(turn: &TurnV2, can_regenerate: bool, select_mode: Signal<bool>) -> Element {
  rsx! {
    div { class: "w-full flex flex-col gap-3",
      for system in turn.system.iter() {
        div { key: "{system.len()}", class: TEXT_CARD,
          Markdown { text: system.clone() }
        }
      }
      for prompt in turn.prompt.iter() {
        PromptCard { key: "{prompt}", text: prompt.clone(), editable: can_regenerate }
      }
      for (idx, content) in turn.responses.iter().filter_map(|response| response.content.as_ref()).enumerate() {
        div { key: "{idx}", class: TEXT_CARD,
          Markdown { text: content.clone() }
        }
      }
      div { class: "mx-auto flex flex-row items-center gap-2",
        { can_regenerate.then(|| rsx! {
            button {
              class: "w-7 h-7 flex items-center justify-center rounded-full cursor-pointer text-nc-text/50 hover:text-nc-text hover:bg-nc-text/10",
              title: "Regenerate response",
              onclick: move |_| engine::regenerate(),
              svg {
                class: "w-4 h-4",
                view_box: "0 0 24 24",
                fill: "none",
                stroke: "currentColor",
                stroke_width: "2",
                stroke_linecap: "round",
                stroke_linejoin: "round",
                path { d: "M21 12a9 9 0 1 1-9-9c2.52 0 4.93 1 6.74 2.74L21 8" }
                path { d: "M21 3v5h-5" }
              }
            }
        }) }
        { selection_toggle(select_mode) }
      }
    }
  }
}

/// Inputs of one profile-field row (key/value with a danger trash button).
const PROFILE_FIELD_INPUT: &str =
  "min-w-0 flex-1 px-3 py-2 bg-transparent border border-nc-border rounded-sm text-nc-text focus:border-nc-label focus:outline-none placeholder:text-nc-text/40";
/// Textarea variant of the above for lengthy values.
const PROFILE_FIELD_TEXTAREA: &str =
  "min-w-0 flex-1 px-3 py-2 bg-transparent border border-nc-border rounded-sm text-nc-text focus:border-nc-label focus:outline-none resize-none leading-snug";

/// Values longer than this many characters flip from a single-line input
/// into a textarea (and back when they drop below it again).
const VALUE_TEXTAREA_THRESHOLD: usize = 50;

/// Modal editor for a profile's name & key/value fields. Tab order runs
/// name → field keys/values — trash & add buttons are skipped; tabbing off
/// the last field's value auto-appends a fresh pair unless both inputs are
/// empty.
#[derive(Props, Clone, PartialEq)]
struct ProfileDialogProps {
  profile: ProfileV1,
  on_close: EventHandler,
  on_save: EventHandler<ProfileV1>,
}

#[component]
fn ProfileDialog(props: ProfileDialogProps) -> Element {
  let mut name = use_signal(|| props.profile.name.clone());
  let mut fields: Signal<Vec<(String, String)>> = use_signal(|| {
    let mut rows: Vec<(String, String)> =
      props.profile.fields.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
    // Always keep one empty pair around to type a new property into.
    if rows.is_empty() {
      rows.push((String::new(), String::new()));
    }
    rows
  });
  // When set, the freshly appended row's key input takes focus on mount.
  let mut focus_new_key = use_signal(|| false);
  // Row whose value editor just flipped between input & textarea and needs
  // its focus (and caret, parked at the end) restored on mount.
  let mut refocus_value: Signal<Option<usize>> = use_signal(|| None);

  let save_profile = props.profile.clone();
  let on_save = props.on_save;
  let save = move |_| {
    let mut map = HashMap::new();
    for (key, value) in fields.read().iter() {
      let key = key.trim().to_string();
      if !key.is_empty() {
        map.insert(key, value.clone());
      }
    }
    let mut profile = save_profile.clone();
    profile.name = name.read().trim().to_string();
    profile.fields = map;
    profile.updated_at = chrono::Utc::now();
    on_save.call(profile);
  };

  let last = fields.read().len().saturating_sub(1);

  rsx! {
    div {
      class: "fixed inset-0 z-50 flex items-center justify-center bg-black/50 cursor-default",
      onclick: move |_| props.on_close.call(()),
      div {
        class: "flex flex-col gap-4 p-4 w-160 bg-nc-bg border rounded-md border-nc-border shadow-lg",
        onclick: move |ev: Event<MouseData>| ev.stop_propagation(),
        div { class: "text-lg", "Edit profile" }
        Field { label: "Name",
          input {
            class: INPUT_CLASS,
            autofocus: true,
            placeholder: "Profile name",
            value: "{name}",
            oninput: move |ev: FormEvent| name.set(ev.value()),
          }
        }
        div { class: "flex flex-col gap-2",
          for idx in 0..fields.read().len() {
            {
              let (field_key, field_value) = fields.read()[idx].clone();
              let is_last = idx == last;
              let long_value = field_value.chars().count() > VALUE_TEXTAREA_THRESHOLD;
              let mut set_value = move |new: String| {
                let old = fields.read()[idx].1.clone();
                fields.write()[idx].1 = new.clone();
                // When the editor flips between input & textarea, the new
                // element needs its focus (and caret) restored on mount.
                let crossed = (old.chars().count() > VALUE_TEXTAREA_THRESHOLD)
                  != (new.chars().count() > VALUE_TEXTAREA_THRESHOLD);
                if crossed {
                  refocus_value.set(Some(idx));
                }
              };
              let tab_handler = move |ev: KeyboardEvent| {
                if is_last
                  && ev.data.key() == Key::Tab
                  && !ev.data.modifiers().contains(Modifiers::SHIFT)
                {
                  let (key, value) = fields.read()[idx].clone();
                  if !key.trim().is_empty() || !value.trim().is_empty() {
                    ev.prevent_default();
                    fields.write().push((String::new(), String::new()));
                    focus_new_key.set(true);
                  }
                }
              };
              let refocus = move |ev: MountedEvent| async move {
                if *refocus_value.read() == Some(idx) {
                  refocus_value.set(None);
                  let _ = ev.data.set_focus(true).await;
                  dioxus::document::eval(&format!(
                    "var el = document.getElementById('pf-value-{idx}'); if (el) {{ el.setSelectionRange(el.value.length, el.value.length); }}"
                  ));
                }
              };
              rsx! {
                div { class: "flex flex-row items-center gap-2 w-full",
                  input {
                    key: "key{idx}",
                    class: PROFILE_FIELD_INPUT,
                    placeholder: "key",
                    value: "{field_key}",
                    oninput: move |ev: FormEvent| fields.write()[idx].0 = ev.value(),
                    onmounted: move |ev: MountedEvent| async move {
                      if *focus_new_key.read() {
                        focus_new_key.set(false);
                        let _ = ev.data.set_focus(true).await;
                      }
                    },
                  }
                  { if long_value {
                      rsx! {
                        textarea {
                          key: "value{idx}",
                          id: "pf-value-{idx}",
                          class: PROFILE_FIELD_TEXTAREA,
                          rows: "3",
                          placeholder: "value",
                          value: "{field_value}",
                          oninput: move |ev: FormEvent| set_value(ev.value()),
                          onkeydown: tab_handler,
                          onmounted: refocus,
                        }
                      }
                    } else {
                      rsx! {
                        input {
                          key: "value{idx}",
                          id: "pf-value-{idx}",
                          class: PROFILE_FIELD_INPUT,
                          placeholder: "value",
                          value: "{field_value}",
                          oninput: move |ev: FormEvent| set_value(ev.value()),
                          onkeydown: tab_handler,
                          onmounted: refocus,
                        }
                      }
                    } }
                  button {
                    class: "flex-none p-1 flex items-center cursor-pointer text-nc-dangerfg/80 hover:text-nc-dangerfg",
                    title: "Remove property",
                    tabindex: -1,
                    onclick: move |_| {
                      let mut rows = fields.write();
                      rows.remove(idx);
                      // Always keep one empty pair around.
                      if rows.is_empty() {
                        rows.push((String::new(), String::new()));
                      }
                    },
                    svg {
                      class: "w-4 h-4",
                      view_box: "0 0 24 24",
                      fill: "none",
                      stroke: "currentColor",
                      stroke_width: "2",
                      stroke_linecap: "round",
                      stroke_linejoin: "round",
                      path { d: "M3 6h18" }
                      path { d: "M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6" }
                      path { d: "M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2" }
                    }
                  }
                }
              }
            }
          }
        }
        button {
          class: "px-2 py-1 rounded-sm flex flex-row items-center gap-2 self-start cursor-pointer text-nc-label hover:bg-nc-text/10",
          tabindex: -1,
          onclick: move |_| fields.write().push((String::new(), String::new())),
          span { class: "text-base leading-none", "+" }
          span { "Add property" }
        }
        div { class: "flex flex-row gap-2",
          Button { label: "Cancel", variant: BtnVariant::Secondary, on_click: move |_| props.on_close.call(()) }
          Button { label: "Save", variant: BtnVariant::Primary, on_click: save }
        }
      }
    }
  }
}

/// The user prompt of a turn: rendered markdown, editable via click on the
/// last turn (saved on blur). Keyed by its text so forks/regenerations
/// re-initialize the edit draft.
#[component]
fn PromptCard(text: String, editable: bool) -> Element {
  let mut editing = use_signal(|| false);
  let mut draft = use_signal(|| text.clone());
  let click_text = text.clone();
  // Rough vertical size: wrap each line at ~60 columns.
  let rows = draft.read().lines()
    .map(|line| line.chars().count().div_ceil(60).max(1))
    .sum::<usize>()
    .max(1);

  rsx! {
    div { class: PROMPT_CARD,
      { if editable && *editing.read() {
          let original = text.clone();
          rsx! {
            textarea {
              class: "w-full bg-transparent resize-none cursor-text text-nc-text/80 focus:outline-none",
              rows: "{rows}",
              value: "{draft}",
              onmounted: move |ev: MountedEvent| async move {
                let _ = ev.data.set_focus(true).await;
              },
              oninput: move |ev: FormEvent| draft.set(ev.value()),
              onblur: move |_| {
                let content = draft.read().clone();
                // Don't persist an empty prompt — keep the previous one.
                if !content.trim().is_empty() && content != original {
                  spawn(async move { engine::set_last_prompt_async(content).await; });
                }
                editing.set(false);
              },
            }
          }
        } else if editable {
          rsx! {
            div {
              class: "cursor-text",
              onclick: move |_| {
                draft.set(click_text.clone());
                editing.set(true);
              },
              Markdown { text: text.clone() }
            }
          }
        } else {
          rsx! { Markdown { text } }
        } }
    }
  }
}

fn stream_card(stream: &StreamState) -> Element {
  rsx! {
    div { class: "w-full flex flex-col gap-3",
      div { class: TEXT_CARD,
        Markdown { text: stream.prompt.clone() }
      }
      { if stream.activity {
          rsx! { Loading { text: "The DM is taking action ..." } }
        } else if stream.text.is_empty() {
          rsx! { Loading { text: "Thinking ..." } }
        } else {
          rsx! {
            div { class: TEXT_CARD,
              Markdown { text: stream.text.clone() }
            }
          }
        } }
    }
  }
}
