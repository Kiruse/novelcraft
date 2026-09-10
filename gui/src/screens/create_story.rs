use std::collections::{BTreeSet, HashMap};
use std::time::Duration;

use gpui::{AnyElement, Div, Entity, Render, Window, div, text};
use gpui::prelude::*;
use tokio::sync::oneshot;

use super::screen;
use crate::actions::ShowStory;
use crate::error::GuiError;
use crate::{ToastVariant, comp::*};
use crate::text_input::TextInput;
use crate::theme::Theme;
use crate::util::{Loadable, Toastable};
use crate::{Command, CommandBus, StoryId, Toast};

/// Grace period after the last category toggle before inspirations are
/// refetched, so rapid chip clicks coalesce into a single request.
const INSPIRATIONS_DEBOUNCE: Duration = Duration::from_millis(300);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct InspirationCategory {
  id: String,
  label: String,
}

#[derive(Debug, Clone)]
struct Inspiration {
  title: String,
  premise: String,
  /// Ids of the [`InspirationCategory`]s this inspiration belongs to.
  categories: Vec<String>,
}

pub(crate) struct CreateStoryScreen {
  title: Entity<TextInput>,
  premise: Entity<TextInput>,
  categories: Loadable<Result<Vec<InspirationCategory>, GuiError>>,
  inspirations: Loadable<Result<Vec<Inspiration>, GuiError>>,
  selected: BTreeSet<String>,
  /// Bumped on every inspirations (re)fetch; stale responses are discarded.
  inspirations_gen: usize,
}

impl CreateStoryScreen {
  pub fn create(cx: &mut Context<'_, Self>) -> Self {
    let mut screen = Self {
      title: create_text_input(cx, false, "Title"),
      premise: create_text_input(
        cx,
        true,
        "Describe the premise of your story ..."
      ),
      categories: Loadable::Pending,
      inspirations: Loadable::Pending,
      selected: BTreeSet::new(),
      inspirations_gen: 0,
    };
    screen.load_categories(cx);
    screen.fetch_inspirations(cx);
    screen
  }

  fn load_categories(&mut self, cx: &mut Context<'_, Self>) {
    cx.spawn(async move |this, cx| {
      let result = fetch_categories().await;
      this.update(cx, |this, cx| {
        result.report_toast(ToastVariant::Error, cx);
        this.categories = Loadable::Done(result);
        cx.notify();
      }).ok();
    }).detach();
  }

  fn fetch_inspirations(&mut self, cx: &mut Context<'_, Self>) {
    self.inspirations_gen += 1;
    let generation = self.inspirations_gen;
    let selected: Vec<String> = self.selected.iter().cloned().collect();
    cx.spawn(async move |this, cx| {
      let result = fetch_inspirations(&selected).await;
      this.update(cx, |this, cx| {
        if this.inspirations_gen == generation {
          result.report_toast(ToastVariant::Error, cx);
          this.inspirations = Loadable::Done(result);
          cx.notify();
        }
      }).ok();
    }).detach();
  }

  /// Toggles a category filter and refetches the inspirations after a
  /// debounce, discarding the response if another request superseded it.
  fn toggle_category(&mut self, id: &str, cx: &mut Context<'_, Self>) {
    if !self.selected.remove(id) {
      self.selected.insert(id.to_string());
    }

    self.inspirations_gen += 1;
    let generation = self.inspirations_gen;
    let selected: Vec<String> = self.selected.iter().cloned().collect();
    cx.spawn(async move |this, cx| {
      cx.background_executor().timer(INSPIRATIONS_DEBOUNCE).await;
      if !this.update(cx, |this, _| this.inspirations_gen == generation).unwrap_or(false) {
        return;
      }

      let result = fetch_inspirations(&selected).await;
      this.update(cx, |this, cx| {
        if this.inspirations_gen == generation {
          result.report_toast(ToastVariant::Error, cx);
          this.inspirations = Loadable::Done(result);
          cx.notify();
        }
      }).ok();
    }).detach();
  }

  fn submit(&mut self, cx: &mut Context<'_, Self>) {
    let title = self.title.read(cx).value().trim().to_string();
    if title.is_empty() { return };

    let exposition = self.premise.read(cx).value().trim().to_string();

    let (tx, rx) = oneshot::channel();
    cx.global::<CommandBus>().send(Command::CreateSession {
      title,
      exposition,
      reply: tx,
    });
    cx.spawn(async move |_, cx| {
      if let Ok(response) = rx.await {
        match response {
          Some(session) => cx.update(|cx| {
            cx.dispatch_action(&ShowStory(StoryId(session.id)));
          }),
          None => cx.update(|cx| {
            Toast::error("Failed to create Vignette").dispatch(cx);
          }),
        }
      }
    }).detach();
  }

  pub(crate) fn enter(&mut self, cx: &mut Context<'_, Self>) {
    // `exit` resets the inspiration state, so every visit reloads it.
    if self.categories.is_pending() {
      self.load_categories(cx);
    }
    if self.inspirations.is_pending() {
      self.fetch_inspirations(cx);
    }
  }

  pub(crate) fn exit(&mut self, cx: &mut Context<'_, Self>) {
    self.title.update(cx, |n, cx| n.reset(cx));
    self.premise.update(cx, |n, cx| n.reset(cx));
    // Invalidate in-flight fetches so they can't repopulate stale data.
    self.inspirations_gen += 1;
    self.inspirations = Loadable::Pending;
    self.categories = Loadable::Pending;
    self.selected.clear();
  }

  fn categories_ui(&self, theme: &Theme, cx: &Context<'_, Self>) -> AnyElement {
    let Loadable::Done(res) = &self.categories else {
      return loading_text("categories-loading", "Loading categories ...")
        .into_any_element();
    };

    match res {
      Ok(categories) => div()
        .flex()
        .flex_wrap()
        .gap_2()
        .w_full()
        .children(categories.iter().map(|category| {
          chip(
            format!("chip-category-{}", category.id),
            text!(category.label.clone()),
          )
          .select(theme, self.selected.contains(&category.id))
          .into_element()
          .on_click({
            let id = category.id.clone();
            cx.listener(move |this, _, _, cx| this.toggle_category(&id, cx))
          })
        }))
        .into_any_element(),
      Err(_) =>
        text!("Failed to query inspirations")
          .into_any_element()
    }
  }

  fn inspirations_ui(&self, theme: &Theme) -> AnyElement {
    let Loadable::Done(res) = &self.inspirations else {
      return loading_text("inspirations-loading", "Loading inspirations ...")
        .into_any_element();
    };

    match res {
      Ok(inspirations) if inspirations.is_empty() =>
        text!("No inspirations found")
          .into_any_element(),
      Ok(inspirations) => {
        let labels = self.category_labels();
        div()
          .flex()
          .flex_col()
          .gap_2()
          .w_full()
          .children(inspirations.iter().map(|inspiration| {
            inspiration_card(theme, inspiration, &labels)
          }))
          .into_any_element()
      }
      Err(_) =>
        text!("Failed to query inspirations")
          .into_any_element()
    }
  }

  fn category_labels(&self) -> HashMap<&str, &str> {
    match &self.categories {
      Loadable::Done(Ok(categories)) => categories
        .iter()
        .map(|category| (category.id.as_str(), category.label.as_str()))
        .collect(),
      _ => HashMap::new(),
    }
  }
}

impl Render for CreateStoryScreen {
  fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    let theme = cx.global::<Theme>();

    let title_valid = cx.read_entity(&self.title, |t, _| !t.value().trim().is_empty());
    let premise_valid = cx.read_entity(&self.premise, |t, _| !t.value().trim().is_empty());
    let valid = title_valid && premise_valid;

    screen(text!("Create Vignette")).closable()
      .child(field(&theme, "Title", &self.title))
      .child(field(&theme, "Premise", &self.premise))
      .child(button("btn-create", text!("Create"))
        .primary(&theme)
        .disable(!valid)
        .into_element()
        .on_click(cx.listener(|this, _, _, cx| this.submit(cx))))
      .child(subtitle(text!("Get Inspired")))
      .child(self.categories_ui(&theme, cx))
      .child(self.inspirations_ui(&theme))
  }
}

fn inspiration_card(
  theme: &Theme,
  inspiration: &Inspiration,
  labels: &HashMap<&str, &str>,
) -> Div {
  div()
    .w_full()
    .p_3()
    .flex()
    .flex_col()
    .gap_1()
    .border_1()
    .border_color(theme.border)
    .rounded_sm()
    .child(div().text_lg().child(text!(inspiration.title.clone())))
    .child(div().child(text!(inspiration.premise.clone())))
    .child(div()
      .flex()
      .flex_wrap()
      .gap_1()
      .children(inspiration.categories.iter().map(|category| {
        let label = labels
          .get(category.as_str())
          .copied()
          .unwrap_or(category.as_str());
        div()
          .px_2()
          .py_0p5()
          .text_sm()
          .text_color(theme.label)
          .border_1()
          .border_color(theme.border)
          .rounded_full()
          .child(text!(label.to_string()))
      })))
}

// TODO: Replace the mocks with REST calls once the inspiration service exists.

async fn fetch_categories() -> Result<Vec<InspirationCategory>, GuiError> {
  Ok(vec![
    InspirationCategory { id: "fantasy".into(), label: "Fantasy".into() },
    InspirationCategory { id: "sci-fi".into(), label: "Sci-Fi".into() },
    InspirationCategory { id: "mystery".into(), label: "Mystery".into() },
    InspirationCategory { id: "romance".into(), label: "Romance".into() },
    InspirationCategory { id: "horror".into(), label: "Horror".into() },
    InspirationCategory { id: "slice-of-life".into(), label: "Slice of Life".into() },
  ])
}

/// OR-filters the inspirations by the selected category ids
/// (an empty selection matches everything).
async fn fetch_inspirations(selected: &[String]) -> Result<Vec<Inspiration>, GuiError> {
  let all = vec![
    Inspiration {
      title: "The Last Lighthouse".into(),
      premise: "On a coast where the tide forgot its rhythm, a lone keeper tends a flame that holds back more than just the dark.".into(),
      categories: vec!["fantasy".into(), "horror".into()],
    },
    Inspiration {
      title: "Signal from Kepler-442".into(),
      premise: "The message repeats every 47 hours. Your predecessor decoded half of it — then walked into the desert without a word.".into(),
      categories: vec!["sci-fi".into(), "mystery".into()],
    },
    Inspiration {
      title: "The Marble Game".into(),
      premise: "Every child in Ashfall plays it. Every child wins. Nobody remembers what they wagered.".into(),
      categories: vec!["mystery".into(), "horror".into()],
    },
    Inspiration {
      title: "Letters to No One".into(),
      premise: "A postal clerk starts answering the letters addressed to a house that burned down thirty years ago.".into(),
      categories: vec!["romance".into(), "slice-of-life".into()],
    },
    Inspiration {
      title: "Sunday at the Noodle Bar".into(),
      premise: "Seven regulars, one bowl left, and the longest rain the city has ever seen.".into(),
      categories: vec!["slice-of-life".into()],
    },
    Inspiration {
      title: "The Cartographer's Debt".into(),
      premise: "She maps places that don't exist yet, and something is collecting on every mile she invents.".into(),
      categories: vec!["fantasy".into(), "sci-fi".into()],
    },
  ];

  if selected.is_empty() {
    return Ok(all);
  }
  Ok(all.into_iter().filter(|inspiration| {
    inspiration.categories.iter().any(|category| selected.contains(category))
  }).collect())
}
