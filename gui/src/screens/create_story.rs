use std::collections::BTreeSet;
use std::time::Duration;

use dioxus::prelude::*;
use serde::Deserialize;

use crate::api::get_json;
use crate::comps::{Button, BtnVariant, Chip, Field, Loading, Markdown, Screen, INPUT_CLASS};
use crate::engine;
use crate::error::GuiError;
use crate::screens::Route;
use crate::toast::{push_toast, Toast};

/// Grace period after the last tag toggle before inspirations are
/// refetched, so rapid chip clicks coalesce into a single request.
const INSPIRATIONS_DEBOUNCE: Duration = Duration::from_millis(300);

#[derive(Debug, Clone, PartialEq, Deserialize)]
struct Inspiration {
  title: String,
  premise: String,
  /// Tags this inspiration is filed under.
  tags: Vec<String>,
}

#[component]
pub fn CreateStory() -> Element {
  let mut title = use_signal(String::new);
  let mut premise = use_signal(String::new);
  let mut direction = use_signal(String::new);
  let mut tags: Signal<Option<Result<Vec<String>, String>>> = use_signal(|| None);
  let inspirations: Signal<Option<Result<Vec<Inspiration>, String>>> = use_signal(|| None);
  let mut selected: Signal<BTreeSet<String>> = use_signal(BTreeSet::new);
  // Bumped on every inspirations (re)fetch; stale responses are discarded.
  let mut inspirations_gen = use_signal(|| 0usize);

  use_hook(move || {
    spawn(async move {
      let result = fetch_tags().await;
      if let Err(err) = &result {
        push_toast(Toast::error(format!("Failed to load tags: {err}")));
      }
      tags.set(Some(result));
    });
    spawn(refetch_inspirations(selected, inspirations_gen, inspirations));
  });

  let submit = move |_| {
    let title_text = title.read().trim().to_string();
    if title_text.is_empty() {
      return;
    }
    let exposition = premise.read().trim().to_string();
    let direction = direction.read().trim().to_string();

    let nav = navigator();
    spawn(async move {
      // Failures are toasted by the engine layer.
      if let Ok(session) = engine::create_session(title_text, exposition, direction).await {
        nav.push(Route::StoryOverview { id: session.id });
      }
    });
  };

  let valid = !title.read().trim().is_empty() && !premise.read().trim().is_empty();

  rsx! {
    Screen { title: "Create Vignette", closable: true,
      Field { label: "Title",
        input {
          class: INPUT_CLASS,
          placeholder: "Title",
          value: "{title}",
          oninput: move |ev: FormEvent| title.set(ev.value()),
        }
      }
      Field { label: "Premise",
        textarea {
          class: INPUT_CLASS,
          rows: 4,
          placeholder: "Describe the premise of your story ...",
          value: "{premise}",
          oninput: move |ev: FormEvent| premise.set(ev.value()),
        }
      }
      Field { label: "Creative Direction",
        textarea {
          class: INPUT_CLASS,
          rows: 3,
          placeholder: "Optional — guide the storyteller's style & behavior (e.g. tone, pacing, content boundaries) ...",
          value: "{direction}",
          oninput: move |ev: FormEvent| direction.set(ev.value()),
        }
      }
      Button { label: "Create", variant: BtnVariant::Primary, disabled: !valid, on_click: submit }
      div { class: "text-2xl", "Get Inspired" }

      { match &*tags.read() {
          None => rsx! { Loading { text: "Loading tags ..." } },
          Some(Err(_)) => rsx! { "Failed to query tags" },
          Some(Ok(list)) =>
            rsx! {
              div { class: "flex flex-wrap gap-2 w-full",
                for tag in list.iter().cloned() {
                  Chip {
                    key: "{tag}",
                    label: tag.clone(),
                    selected: selected.read().contains(&tag),
                    on_click: move |_| {
                      // Toggling a tag refetches the inspirations after a
                      // debounce; stale responses are discarded by the
                      // generation counter.
                      if !selected.write().remove(&tag) {
                        selected.write().insert(tag.clone());
                      }
                      *inspirations_gen.write() += 1;
                      spawn(refetch_inspirations(selected, inspirations_gen, inspirations));
                    },
                  }
                }
              }
            },
      } }

      { match &*inspirations.read() {
          None => rsx! { Loading { text: "Loading inspirations ..." } },
          Some(Err(_)) => rsx! { "Failed to query inspirations" },
          Some(Ok(list)) if list.is_empty() => rsx! { "No inspirations found" },
          Some(Ok(list)) =>
            rsx! {
              div { class: "flex flex-col gap-2 w-full",
                for inspiration in list.iter() {
                  InspirationCard {
                    key: "{inspiration.title}",
                    inspiration: inspiration.clone(),
                  }
                }
              }
            },
      } }
    }
  }
}

/// Debounced inspirations fetch, discarding the response if another
/// request superseded it.
async fn refetch_inspirations(
  selected: Signal<BTreeSet<String>>,
  inspirations_gen: Signal<usize>,
  mut inspirations: Signal<Option<Result<Vec<Inspiration>, String>>>,
) {
  let generation = *inspirations_gen.read();
  crate::time::sleep(INSPIRATIONS_DEBOUNCE).await;
  if *inspirations_gen.read() != generation {
    return;
  }

  let sel: Vec<String> = selected.read().iter().cloned().collect();
  let result = fetch_inspirations_api(&sel).await;
  if *inspirations_gen.read() == generation {
    if let Err(err) = &result {
      push_toast(Toast::error(format!("Failed to fetch inspirations: {err}")));
    }
    inspirations.set(Some(result));
  }
}

/// The service OR-filters by the comma-joined selected tags via the `tags`
/// query param (an empty selection matches everything).
async fn fetch_inspirations_api(selected: &[String]) -> Result<Vec<Inspiration>, String> {
  let query: Vec<(String, String)> = if selected.is_empty() {
    vec![]
  } else {
    vec![("tags".to_string(), selected.join(","))]
  };
  get_json::<Vec<Inspiration>>("/inspirations", &query)
    .await
    .map_err(|e: GuiError| e.to_string())
}

async fn fetch_tags() -> Result<Vec<String>, String> {
  get_json::<Vec<String>>("/inspirations/tags", &[])
    .await
    .map_err(|e: GuiError| e.to_string())
}

#[component]
fn InspirationCard(inspiration: Inspiration) -> Element {
  rsx! {
    div { class: "w-full p-3 flex flex-col gap-1 border rounded-sm border-nc-border",
      div { class: "text-lg", { inspiration.title.clone() } }
      Markdown { text: inspiration.premise.clone(), class: "text-sm/relaxed" }
      div { class: "flex flex-wrap gap-1",
        for tag in inspiration.tags.iter() {
          div {
            key: "{tag}",
            class: "px-2 py-0.5 text-sm text-nc-label border rounded-full border-nc-border",
            { tag.clone() }
          }
        }
      }
    }
  }
}
