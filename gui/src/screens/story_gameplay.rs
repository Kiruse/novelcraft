use dioxus::prelude::*;
use novelcraft_engine::game::pages::PageV1;

use crate::comps::{CloseButton, Loading, INPUT_CLASS};
use crate::engine;
use crate::state::{StreamState, PAGE, PAGE_INDEX, PROFILES, SESSION, STREAM};

/// Swipe distance (in logical pixels) required to flip to the adjacent page.
const SWIPE_THRESHOLD: f64 = 60.0;
/// Width of the toggleable game state side bar.
const SIDEBAR_WIDTH_PX: f64 = 280.0;

#[component]
pub fn StoryGameplay(id: String) -> Element {
  let mut input = use_signal(String::new);
  let popover_open = use_signal(|| false);
  let mut sidebar_open = use_signal(|| false);
  // Horizontal start position of the active swipe gesture, if any.
  let mut swipe_start: Signal<Option<f64>> = use_signal(|| None);

  // Load the session into the engine, then open its last page.
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
    && engine::session_page_count().is_some()
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

    let Some(count) = engine::session_page_count() else { return };
    let page_idx = *PAGE_INDEX.read();
    // Fork from the current page only when not already on the last page.
    let from_page = (page_idx + 1 < count).then_some(page_idx);
    engine::prompt(content, from_page);
  };

  rsx! {
    div {
      class: "flex flex-col items-center w-full h-full bg-nc-bg text-nc-text",
      onkeydown: move |ev: KeyboardEvent| {
        // Page navigation via alt+arrows, mirroring the previous global
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
          { page_content_ui() }
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

fn page_content_ui() -> Element {
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

  // Fresh session — there are no pages to fetch yet.
  if session.page_count == 0 {
    return rsx! { "Send your first prompt to begin the story ..." };
  }

  let page_q = PAGE.read();
  let Some(page) = page_q.value() else {
    return if page_q.error().is_some() {
      rsx! { div { class: "text-nc-dangerfg", "Failed to load page" } }
    } else {
      rsx! { Loading { text: "Loading page ..." } }
    };
  };

  if page_is_empty(page) {
    rsx! { "Send your first prompt to begin the story ..." }
  } else {
    page_card(page)
  }
}

fn status_row_ui() -> Element {
  let count = engine::session_page_count();
  let page_index = *PAGE_INDEX.read();
  let streaming = STREAM.read().is_some();
  let show_indicator = !streaming && count.is_some_and(|count| count > 0);
  let show_jump = !streaming && count.is_some_and(|count| count > 0 && page_index + 1 < count);

  rsx! {
    div { class: "w-full flex flex-row justify-center items-center gap-3",
      { show_indicator.then(|| {
          rsx! {
            div { class: "text-sm text-nc-label", "{page_index + 1}/{count.unwrap()}" }
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
  rsx! {
    div {
      class: "flex-none h-full border-l p-4 border-nc-border",
      style: "width: {SIDEBAR_WIDTH_PX}px",
      div { class: "text-sm text-nc-label", "Game state \u{2014} coming soon" }
    }
  }
}

fn page_is_empty(page: &PageV1) -> bool {
  page.prompt.is_none()
    && page.responses.iter().all(|response| response.content.is_none())
}

fn page_card(page: &PageV1) -> Element {
  rsx! {
    div { class: "w-full flex flex-col gap-3",
      for prompt in page.prompt.iter() {
        div {
          class: "w-full p-3 border rounded-sm border-nc-border text-nc-label",
          { prompt.clone() }
        }
      }
      for content in page.responses.iter().filter_map(|response| response.content.as_ref()) {
        div { class: "w-full", { content.clone() } }
      }
    }
  }
}

fn stream_card(stream: &StreamState) -> Element {
  rsx! {
    div { class: "w-full flex flex-col gap-3",
      div { class: "w-full p-3 border rounded-sm border-nc-border text-nc-label",
        { stream.prompt.clone() }
      }
      { if stream.activity {
          rsx! { Loading { text: "The DM is taking action ..." } }
        } else if stream.text.is_empty() {
          rsx! { Loading { text: "Thinking ..." } }
        } else {
          rsx! { div { class: "w-full", { stream.text.clone() } } }
        } }
    }
  }
}
