use dioxus::prelude::*;

use crate::comps::{Button, BtnVariant, Loading, Screen};
use crate::engine;
use crate::screens::Route;
use crate::state::SESSIONS;

#[component]
pub fn StoryOverview(id: String) -> Element {
  // The sessions cache may not know a freshly created session yet —
  // refresh it before resolving this story's details.
  use_hook(engine::refresh_sessions);

  let nav = navigator();
  let session = SESSIONS
    .read()
    .value()
    .and_then(|sessions| sessions.iter().find(|session| session.id == id).cloned());

  rsx! {
    Screen { title: "Story Overview", closable: true,
      BannerPlaceholder {}
      { match session {
          None => rsx! { Loading { text: "Loading ..." } },
          Some(session) =>
            rsx! {
              div { class: "flex flex-col gap-1 w-full",
                div { class: "text-2xl", { session.title.clone() } }
                div { { session.exposition.clone() } }
              }
            },
      } }
      Button {
        label: "Play",
        variant: BtnVariant::Primary,
        on_click: move |_| {
          nav.push(Route::StoryGameplay { id: id.clone() });
        },
      }
    }
  }
}

/// Stand-in for the story's cover image banner.
#[component]
fn BannerPlaceholder() -> Element {
  rsx! {
    div {
      class: "w-full h-55 flex items-center justify-center border rounded-md border-nc-border bg-nc-text/5 text-nc-label",
      "No cover image"
    }
  }
}
