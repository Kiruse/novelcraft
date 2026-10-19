use dioxus::prelude::*;

use crate::comps::{Button, BtnVariant, Loading, Markdown, Screen};
use crate::engine;
use crate::screens::{Route, go_back};
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

  let mut confirm_delete = use_signal(|| false);
  let title_for_dialog = session
    .as_ref()
    .map(|session| session.title.trim().to_string())
    .filter(|title| !title.is_empty());
  let play_id = id.clone();

  rsx! {
    Screen { title: "Story Overview", closable: true,
      BannerPlaceholder {}
      { match session {
          None => rsx! { Loading { text: "Loading ..." } },
          Some(session) =>
            rsx! {
              div { class: "flex flex-col gap-1 w-full",
                div { class: "text-2xl", { session.title.clone() } }
                Markdown { text: session.exposition.clone() }
              }
            },
      } }
      div { class: "flex flex-row gap-2 w-full",
        Button {
          label: "Play",
          variant: BtnVariant::Primary,
          on_click: move |_| {
            nav.push(Route::StoryGameplay { id: play_id.clone() });
          },
        }
        Button {
          label: "Delete",
          variant: BtnVariant::DangerSecondary,
          on_click: move |_| confirm_delete.set(true),
        }
      }

      { confirm_delete.read().then(|| {
          let dialog_id = id.clone();
          rsx! {
            ConfirmDialog {
              message: match &title_for_dialog {
                Some(title) => format!("Delete \"{title}\"?"),
                None => "Delete this story?".to_string(),
              },
              on_confirm: move |_| {
                confirm_delete.set(false);
                let session_id = dialog_id.clone();
                spawn(async move {
                  if engine::delete_session(session_id).await.is_ok() {
                    go_back(Route::Home {});
                  }
                });
              },
              on_cancel: move |_| confirm_delete.set(false),
            }
          }
      }) }
    }
  }
}

/// Modal confirmation overlay. Confirming or cancelling (button, or clicking
/// the backdrop) dismisses the dialog.
#[derive(Props, Clone, PartialEq)]
struct ConfirmDialogProps {
  message: String,
  on_confirm: EventHandler,
  on_cancel: EventHandler,
}

#[component]
fn ConfirmDialog(props: ConfirmDialogProps) -> Element {
  rsx! {
    div {
      class: "fixed inset-0 z-50 flex items-center justify-center bg-black/50 cursor-default",
      onclick: move |_| props.on_cancel.call(()),
      div {
        class: "flex flex-col gap-4 p-4 w-96 bg-nc-bg border rounded-md border-nc-border shadow-lg",
        onclick: move |ev: Event<MouseData>| ev.stop_propagation(),
        div { class: "flex flex-col gap-1",
          div { class: "text-lg", { props.message.clone() } }
          div { class: "text-sm text-nc-label",
            "This permanently removes the story, including all of its memory. This cannot be undone."
          }
        }
        div { class: "flex flex-row gap-2",
          Button {
            label: "Cancel",
            variant: BtnVariant::Secondary,
            on_click: move |_| props.on_cancel.call(()),
          }
          Button {
            label: "Delete",
            variant: BtnVariant::DangerPrimary,
            on_click: move |_| props.on_confirm.call(()),
          }
        }
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
