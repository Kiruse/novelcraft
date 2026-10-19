use dioxus::prelude::*;
use novelcraft_engine::game::module::memory::MemoryState;
use novelcraft_engine::game::turns::TurnV2;

use crate::comps::{CloseButton, Loading, Markdown, INPUT_CLASS};
use crate::engine;
use crate::state::{StreamState, PROFILES, SESSION, STREAM, TURN, TURN_INDEX};

/// Swipe distance (in logical pixels) required to flip to the adjacent turn.
const SWIPE_THRESHOLD: f64 = 60.0;
/// Width of the toggleable story memory side bar.
const SIDEBAR_WIDTH_PX: f64 = 280.0;

#[component]
pub fn StoryGameplay(id: String) -> Element {
  let mut input = use_signal(String::new);
  let popover_open = use_signal(|| false);
  let mut sidebar_open = use_signal(|| false);
  // Horizontal start position of the active swipe gesture, if any.
  let mut swipe_start: Signal<Option<f64>> = use_signal(|| None);

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

    let Some(count) = engine::session_turn_count() else { return };
    let turn_idx = *TURN_INDEX.read();
    // Fork from the current turn only when not already on the last turn.
    let from_turn = (turn_idx + 1 < count).then_some(turn_idx);
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
          class: "absolute left-4 text-xl cursor-pointer hover:opacity-70",
          onclick: move |_| sidebar_open.toggle(),
          "\u{2630}"
        }
        div { class: "text-3xl", { title_text } }
        CloseButton {}
      }

      div { class: "flex flex-row flex-1 min-h-0 w-full",
        div {
          class: "flex-1 min-w-0 h-full flex flex-col gap-3 p-4 overflow-y-auto select-none",
          onpointerdown: move |ev: PointerEvent| {
            swipe_start.set(Some(ev.data.client_coordinates().x));
          },
          onpointermove: move |ev: PointerEvent| {
            let Some(start) = *swipe_start.read() else { return };
            let dx = ev.data.client_coordinates().x - start;
            if dx <= -SWIPE_THRESHOLD {
              swipe_start.set(None);
              engine::navigate(1);
            } else if dx >= SWIPE_THRESHOLD {
              swipe_start.set(None);
              engine::navigate(-1);
            }
          },
          onpointerup: move |_| swipe_start.set(None),
          onpointerleave: move |_| swipe_start.set(None),
          { turn_content_ui() }
        }
        { sidebar_open.read().then(|| rsx! { Sidebar {} }) }
      }

      { status_row_ui() }

      div { class: "w-full flex flex-row items-end gap-2 p-4",
        { avatar_ui(popover_open) }
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
    }
  }
}

fn turn_content_ui() -> Element {
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
  if session.turn_count() == 0 {
    return rsx! { "Send your first prompt to begin the story ..." };
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
    rsx! { "Send your first prompt to begin the story ..." }
  } else {
    turn_card(turn)
  }
}

fn status_row_ui() -> Element {
  let count = engine::session_turn_count();
  let turn_index = *TURN_INDEX.read();
  let streaming = STREAM.read().is_some();
  let show_indicator = !streaming && count.is_some_and(|count| count > 0);
  let show_jump = !streaming && count.is_some_and(|count| count > 0 && turn_index + 1 < count);

  rsx! {
    div { class: "w-full flex flex-row justify-center items-center gap-3",
      { show_indicator.then(|| {
          rsx! {
            div { class: "text-sm text-nc-label", "{turn_index + 1}/{count.unwrap()}" }
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

fn avatar_ui(mut popover_open: Signal<bool>) -> Element {
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
      { popover_open.read().then(|| profile_popover_ui(popover_open)) }
    }
  }
}

fn profile_popover_ui(mut popover_open: Signal<bool>) -> Element {
  let profiles_q = PROFILES.read();
  let active_id = active_profile_id();

  let list_ui = match profiles_q.value() {
    Some(profiles) if !profiles.profiles.is_empty() =>
      rsx! {
        div { class: "flex flex-col gap-1",
          for profile in profiles.profiles.iter() {
            {
              let is_active = active_id.as_deref() == Some(profile.id.as_str());
              let pid = profile.id.clone();
              rsx! {
                button {
                  key: "{profile.id}",
                  class: "px-2 py-1 rounded-sm flex flex-row justify-between items-center cursor-pointer hover:bg-nc-text/10",
                  onclick: move |_| {
                    popover_open.set(false);
                    let pid = pid.clone();
                    spawn(async move {
                      engine::switch_profile(pid).await;
                    });
                  },
                  span { { profile.name.clone() } }
                  { is_active.then(|| rsx! { span { class: "text-nc-success", "\u{2713}" } }) }
                }
              }
            }
          }
        }
      },
    _ =>
      rsx! {
        div { class: "p-2 text-sm text-nc-label", "No profiles yet" }
      },
  };

  rsx! {
    div {
      class: "absolute bottom-full left-0 mb-2 w-56 max-h-70 overflow-y-auto flex flex-col gap-1 p-2 bg-nc-bg border rounded-sm border-nc-border shadow-lg",
      { list_ui }
    }
  }
}

#[component]
fn Sidebar() -> Element {
  // The sidebar is only mounted while open, so this fetches on every open.
  let mut memory: Signal<Option<Result<MemoryState, String>>> = use_signal(|| None);
  use_hook(move || {
    spawn(async move {
      let res = engine::memory_async().await.map_err(|err| err.to_string());
      memory.set(Some(res));
    });
  });

  let exposition = SESSION.read().value()
    .map(|session| session.exposition.clone())
    .unwrap_or_default();
  let memory_body = match memory.read().as_ref() {
    None => rsx! { Loading { text: "Loading memory ..." } },
    Some(Err(err)) => rsx! { div { class: "text-nc-dangerfg text-sm", "{err}" } },
    Some(Ok(mem)) => memory_ui(&exposition, mem),
  };

  rsx! {
    div {
      class: "flex-none h-full border-l p-4 border-nc-border overflow-y-auto",
      style: "width: {SIDEBAR_WIDTH_PX}px",
      div { class: "text-sm text-nc-label mb-3", "Story memory" }
      { memory_body }
    }
  }
}

fn memory_ui(exposition: &str, mem: &MemoryState) -> Element {
  let premise_empty = exposition.trim().is_empty();
  let summary_empty = mem.summary.trim().is_empty();

  rsx! {
    div { class: "flex flex-col gap-4 text-sm",
      { (!premise_empty).then(|| rsx! {
        div { class: "flex flex-col gap-1",
          div { class: "text-nc-label", "Premise" }
          div { class: "whitespace-pre-wrap", { exposition.to_string() } }
        }
      }) }
      { (!summary_empty).then(|| rsx! {
        div { class: "flex flex-col gap-1",
          div { class: "text-nc-label", "Story so far" }
          div { class: "whitespace-pre-wrap", { mem.summary.clone() } }
        }
      }) }
      { (!mem.chapters.is_empty()).then(|| rsx! {
        div { class: "flex flex-col gap-2",
          div { class: "text-nc-label", "Chapters" }
          for chapter in mem.chapters.iter().filter(|ch| !ch.title.trim().is_empty() || !ch.notes.trim().is_empty()) {
            {
              let title = if chapter.title.trim().is_empty() {
                "Current chapter".to_string()
              } else {
                chapter.title.clone()
              };
              rsx! {
                div {
                  key: "{chapter.title}",
                  class: "flex flex-col gap-1",
                  div { class: "font-medium", { title } }
                  { (!chapter.notes.trim().is_empty()).then(|| rsx! {
                    div { class: "text-nc-label whitespace-pre-wrap", { chapter.notes.trim().to_string() } }
                  }) }
                }
              }
            }
          }
        }
      }) }
      { (premise_empty && summary_empty && mem.chapters.is_empty()).then(|| rsx! {
        div { class: "text-nc-label", "The story has no memory yet." }
      }) }
    }
  }
}

fn turn_is_empty(turn: &TurnV2) -> bool {
  turn.prompt.is_none()
    && turn.responses.iter().all(|response| response.content.is_none())
}

fn turn_card(turn: &TurnV2) -> Element {
  rsx! {
    div { class: "w-full flex flex-col gap-3",
      for system in turn.system.iter() {
        Markdown { key: "{system.len()}", text: system.clone(), class: "w-full" }
      }
      for prompt in turn.prompt.iter() {
        div {
          class: "w-full p-3 border rounded-sm border-nc-border",
          Markdown { text: prompt.clone() }
        }
      }
      for (idx, content) in turn.responses.iter().filter_map(|response| response.content.as_ref()).enumerate() {
        Markdown { key: "{idx}", text: content.clone(), class: "w-full" }
      }
    }
  }
}

fn stream_card(stream: &StreamState) -> Element {
  rsx! {
    div { class: "w-full flex flex-col gap-3",
      div { class: "w-full p-3 border rounded-sm border-nc-border",
        Markdown { text: stream.prompt.clone() }
      }
      { if stream.activity {
          rsx! { Loading { text: "The DM is taking action ..." } }
        } else if stream.text.is_empty() {
          rsx! { Loading { text: "Thinking ..." } }
        } else {
          rsx! { Markdown { text: stream.text.clone(), class: "w-full" } }
        } }
    }
  }
}
