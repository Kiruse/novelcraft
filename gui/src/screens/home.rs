use std::time::Duration;

use gpui::{
  Action, Animation, AnimationExt, AnyElement, Div, InteractiveElement, Render, Stateful, Task, Window, div, pulsating_between, text,
};
use gpui::prelude::*;
use novelcraft_engine::game::session::SessionV1;

use super::screen;
use crate::actions::{CreateStory, ShowStory};
use crate::comp::*;
use crate::theme::Theme;
use crate::{EngineQuery, StoryId};

pub(crate) struct HomeScreen {
  /// Keeps the query watcher alive — cancelled when the screen drops.
  #[allow(dead_code)]
  task: Task<()>,
}

impl HomeScreen {
  pub fn create(cx: &mut Context<'_, Self>) -> Self {
    // Marked unchanged — the watcher fires on the refresh triggered below.
    let mut rx_sessions = cx.global::<EngineQuery<Vec<SessionV1>>>().refresh(cx);

    let task = cx.spawn(async move |this, cx| {
      while let Ok(()) = rx_sessions.changed().await {
        cx.update(|cx| {
          cx.notify(this.entity_id());
        });
      }
    });

    Self { task }
  }

  pub fn enter(&mut self, cx: &mut Context<'_, Self>) {
    cx.global::<EngineQuery<Vec<SessionV1>>>().refresh(cx);
  }

  /// Renders the sessions list for the current [`EngineQueryResult`] state:
  /// loading (empty), error, stale (value + error), and fresh values —
  /// empty or not.
  fn sessions_ui(&self, theme: &Theme, cx: &Context<'_, Self>) -> AnyElement {
    let result = cx.global::<EngineQuery<Vec<SessionV1>>>().curr();
    let (sessions, error) = (result.value(), result.error());

    // A value paired with an error is stale — surface that alongside it.
    let stale_note = result.is_stale().then(|| {
      div()
        .text_sm()
        .text_color(theme.danger_fg)
        .child(text!("Failed to refresh — showing the last known sessions"))
        .into_any_element()
    });

    let list_ui: AnyElement = match (sessions, error) {
      (None, None) =>
        div()
          .child(text!("Loading ..."))
          .with_animation(
            "sessions-loading",
            Animation::new(Duration::from_secs(1))
              .repeat()
              .with_easing(pulsating_between(0.2, 1.0)),
            |loading, delta| loading.opacity(delta),
          )
          .into_any_element(),
      (None, Some(_)) =>
        div()
          .text_color(theme.danger_fg)
          .child(text!("Failed to load sessions"))
          .into_any_element(),
      (Some(sessions), _) if sessions.is_empty() =>
        text!("No sessions yet").into_any_element(),
      (Some(sessions), _) =>
        div()
          .flex()
          .flex_col()
          .gap_2()
          .w_full()
          .children(sessions.iter().map(|session| {
            let story_id = StoryId(session.id.clone());
            session_card(theme, session).on_click(cx.listener(move |_, _, window, cx| {
              window.dispatch_action(ShowStory(story_id.clone()).boxed_clone(), cx)
            }))
          }))
          .into_any_element(),
    };

    div()
      .flex()
      .flex_col()
      .gap_2()
      .w_full()
      .children(stale_note)
      .child(list_ui)
      .into_any_element()
  }
}

impl Render for HomeScreen {
  fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    let theme = cx.global::<Theme>();

    screen(text!("NovelCraft"))
      .child(create_vignette(theme))
      .child(subtitle(text!("Sessions")))
      .child(self.sessions_ui(&theme, cx))
  }
}

fn create_vignette(theme: &Theme) -> Stateful<Div> {
  div()
    .id("create-vignette")
    .w_full()
    .p_6()
    .flex()
    .flex_col()
    .gap_2()
    .border_1()
    .border_color(theme.border)
    .rounded_md()
    .cursor_pointer()
    .hover(|style| style.border_color(theme.label))
    .on_click(|_, window, cx| window.dispatch_action(CreateStory.boxed_clone(), cx))
    .child(
      div()
        .flex()
        .items_center()
        .justify_center()
        .gap_2()
        .text_2xl()
        .child(text!("+"))
        .child(text!("Create new Vignette"))
    )
    .child(
      div()
        .text_color(theme.label)
        .child(text!("Vignettes are free-form stories that don't follow any template. They resemble most closely what you get from other story generator platforms."))
    )
}

fn session_card(theme: &Theme, session: &SessionV1) -> Stateful<Div> {
  div()
    .id(session.id.clone())
    .w_full()
    .p_3()
    .flex()
    .flex_col()
    .gap_1()
    .border_1()
    .border_color(theme.border)
    .rounded_sm()
    .cursor_pointer()
    .hover(|style| style.border_color(theme.label))
    .child(div().text_lg().child(text!(session.title.clone())))
    .child(
      div()
        .text_sm()
        .text_color(theme.label)
        .child(text!(session
          .updated_at
          .with_timezone(&chrono::Local)
          .format("%Y-%m-%d %H:%M")
          .to_string()))
    )
    .child(div().child(text!(session.exposition.clone())))
}
