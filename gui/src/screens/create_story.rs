use std::collections::BTreeSet;
use std::time::Duration;

use gpui::{AnyElement, Div, Entity, Render, Window, div, text};
use gpui::prelude::*;
use novelcraft_engine::game::session::SessionV1;
use serde::Deserialize;

use super::screen;
use crate::actions::ShowStory;
use crate::api::get_json;
use crate::{EngineQuery, comp::*};
use crate::error::GuiError;
use crate::text_input::TextInput;
use crate::theme::Theme;
use crate::util::Toastable;
use crate::{Command, StoryId};

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

pub(crate) struct CreateStoryScreen {
  title: Entity<TextInput>,
  premise: Entity<TextInput>,
  tags: Option<Result<Vec<String>, GuiError>>,
  inspirations: Option<Result<Vec<Inspiration>, GuiError>>,
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
      tags: None,
      inspirations: None,
      selected: BTreeSet::new(),
      inspirations_gen: 0,
    };
    screen.load_tags(cx);
    screen.fetch_inspirations(cx);
    screen
  }

  fn load_tags(&mut self, cx: &mut Context<'_, Self>) {
    cx.spawn(async move |this, cx| {
      let result = fetch_tags().await;
      this.update(cx, |this, cx| {
        this.tags = Some(result.error_toast(cx));
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
          this.inspirations = Some(result.error_toast(cx));
          cx.notify();
        }
      }).ok();
    }).detach();
  }

  /// Toggles a tag filter and refetches the inspirations after a
  /// debounce, discarding the response if another request superseded it.
  fn toggle_tag(&mut self, tag: &str, cx: &mut Context<'_, Self>) {
    if !self.selected.remove(tag) {
      self.selected.insert(tag.to_string());
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
          this.inspirations = Some(result.error_toast(cx));
          cx.notify();
        }
      }).ok();
    }).detach();
  }

  fn submit(&mut self, cx: &mut Context<'_, Self>) {
    let title = self.title.read(cx).value().trim().to_string();
    if title.is_empty() { return };

    let exposition = self.premise.read(cx).value().trim().to_string();

    Command::CreateSession {
      title,
      exposition,
    }.dispatch(cx);

    // Scheduled after CreateSession on the engine thread, so the awaited
    // refresh observes the freshly created (or failed) session.
    let mut rx_session = cx.global::<EngineQuery<SessionV1>>().refresh(cx);

    cx.spawn(async move |_, cx| {
      let Ok(()) = rx_session.changed().await else { return };
      let result = rx_session.borrow();
      // Only navigate on a fresh, error-free session — failures are
      // toasted by the engine thread.
      if result.error().is_some() { return };
      let Some(id) = result.value().map(|session| StoryId(session.id.clone())) else { return };
      cx.update(|cx| {
        cx.dispatch_action(&ShowStory(id));
      });
    }).detach();
  }

  pub(crate) fn enter(&mut self, cx: &mut Context<'_, Self>) {
    // `exit` resets the inspiration state, so every visit reloads it.
    if self.tags.is_none() {
      self.load_tags(cx);
    }
    if self.inspirations.is_none() {
      self.fetch_inspirations(cx);
    }
  }

  pub(crate) fn exit(&mut self, cx: &mut Context<'_, Self>) {
    self.title.update(cx, |n, cx| n.reset(cx));
    self.premise.update(cx, |n, cx| n.reset(cx));
    // Invalidate in-flight fetches so they can't repopulate stale data.
    self.inspirations_gen += 1;
    self.inspirations = None;
    self.tags = None;
    self.selected.clear();
  }

  fn tags_ui(&self, theme: &Theme, cx: &Context<'_, Self>) -> AnyElement {
    let Some(res) = &self.tags else {
      return loading_text("tags-loading", "Loading tags ...")
        .into_any_element();
    };

    match res {
      Ok(tags) => div()
        .flex()
        .flex_wrap()
        .gap_2()
        .w_full()
        .children(tags.iter().map(|tag| {
          chip(
            format!("chip-tag-{}", tag),
            text!(tag.clone()),
          )
          .select(theme, self.selected.contains(tag))
          .into_element()
          .on_click({
            let tag = tag.clone();
            cx.listener(move |this, _, _, cx| this.toggle_tag(&tag, cx))
          })
        }))
        .into_any_element(),
      Err(_) =>
        text!("Failed to query tags")
          .into_any_element()
    }
  }

  fn inspirations_ui(&self, theme: &Theme) -> AnyElement {
    let Some(res) = &self.inspirations else {
      return loading_text("inspirations-loading", "Loading inspirations ...")
        .into_any_element();
    };

    match res {
      Ok(inspirations) if inspirations.is_empty() =>
        text!("No inspirations found")
          .into_any_element(),
      Ok(inspirations) =>
        div()
          .flex()
          .flex_col()
          .gap_2()
          .w_full()
          .children(inspirations.iter().map(|inspiration| {
            inspiration_card(theme, inspiration)
          }))
          .into_any_element(),
      Err(_) =>
        text!("Failed to query inspirations")
          .into_any_element()
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
      .child(self.tags_ui(&theme, cx))
      .child(self.inspirations_ui(&theme))
  }
}

fn inspiration_card(theme: &Theme, inspiration: &Inspiration) -> Div {
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
      .children(inspiration.tags.iter().map(|tag| {
        div()
          .px_2()
          .py_0p5()
          .text_sm()
          .text_color(theme.label)
          .border_1()
          .border_color(theme.border)
          .rounded_full()
          .child(text!(tag.clone()))
      })))
}

async fn fetch_tags() -> Result<Vec<String>, GuiError> {
  get_json("/inspirations/tags", &[]).await
}

/// The service OR-filters by the comma-joined selected tags via the `tags`
/// query param (an empty selection matches everything).
async fn fetch_inspirations(selected: &[String]) -> Result<Vec<Inspiration>, GuiError> {
  let query = if selected.is_empty() {
    vec![]
  } else {
    vec![("tags".to_string(), selected.join(","))]
  };
  get_json("/inspirations", &query).await
}
