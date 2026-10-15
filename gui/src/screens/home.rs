use dioxus::prelude::*;
use novelcraft_engine::game::session::SessionV1;

use super::Route;

use crate::comps::{Loading, Screen};
use crate::engine;
use crate::state::{Query, SESSIONS};

#[component]
pub fn Home() -> Element {
  use_hook(engine::refresh_sessions);

  let sessions_q = SESSIONS.read().clone();

  rsx! {
    Screen { title: "NovelCraft",
      CreateVignette {}
      div { class: "text-2xl", "Sessions" }
      { sessions_ui(&sessions_q) }
    }
  }
}

/// Renders the sessions list for the current query state: loading (empty),
/// error, stale (value + error), and fresh values — empty or not.
fn sessions_ui(q: &Query<Vec<SessionV1>>) -> Element {
  rsx! {
    div { class: "flex flex-col gap-2 w-full",
      { q.is_stale().then(|| {
          rsx! {
            div { class: "text-sm text-nc-dangerfg",
              "Failed to refresh — showing the last known sessions"
            }
          }
      }) }
      { match (q.value(), q.error()) {
          (None, None) =>
            rsx! { Loading { text: "Loading ..." } },
          (None, Some(_)) =>
            rsx! { div { class: "text-nc-dangerfg", "Failed to load sessions" } },
          (Some(sessions), _) if sessions.is_empty() =>
            rsx! { "No sessions yet" },
          (Some(sessions), _) =>
            rsx! {
              for session in sessions.iter() {
                {
                  let updated_label = session
                    .updated_at
                    .with_timezone(&chrono::Local)
                    .format("%Y-%m-%d %H:%M")
                    .to_string();
                  rsx! {
                    SessionCard {
                      key: "{session.id}",
                      id: session.id.clone(),
                      title: session.title.clone(),
                      exposition: session.exposition.clone(),
                      updated_label,
                    }
                  }
                }
              }
            },
      } }
    }
  }
}

#[component]
fn CreateVignette() -> Element {
  let nav = navigator();
  rsx! {
    button {
      class: "w-full p-6 flex flex-col gap-2 border rounded-md border-nc-border cursor-pointer hover:border-nc-label text-left",
      onclick: move |_| {
        nav.push(Route::CreateStory {});
      },
      div { class: "flex items-center justify-center gap-2 text-2xl",
        span { "+" }
        span { "Create new Vignette" }
      }
      div {
        class: "text-nc-label",
        "Vignettes are free-form stories that don't follow any template. They resemble most closely what you get from other story generator platforms."
      }
    }
  }
}

#[component]
fn SessionCard(id: String, title: String, exposition: String, updated_label: String) -> Element {
  let nav = navigator();

  rsx! {
    button {
      key: "{id}",
      class: "w-full p-3 flex flex-col gap-1 border rounded-sm border-nc-border cursor-pointer hover:border-nc-label text-left",
      onclick: move |_| {
        nav.push(Route::StoryOverview { id: id.clone() });
      },
      div { class: "text-lg", { title.clone() } }
      div { class: "text-sm text-nc-label", { updated_label.clone() } }
      div { { exposition.clone() } }
    }
  }
}
