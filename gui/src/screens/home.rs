use std::time::Duration;

use gpui::{
  Action, Animation, AnimationExt, AnyElement, Div, InteractiveElement, Render, Stateful, Window,
  div, pulsating_between, text,
};
use gpui::prelude::*;
use novelcraft_engine::game::session::SessionV1;
use tokio::sync::oneshot;

use crate::actions::{CreateStory, ShowStory};
use crate::comp::*;
use crate::theme::Theme;
use crate::{Command, CommandBus, StoryId};

pub(crate) struct HomeScreen {
  sessions: Option<Vec<SessionV1>>,
}

impl HomeScreen {
  pub fn create(cx: &mut Context<'_, Self>) -> Self {
    let mut screen = Self { sessions: None };
    screen.load(cx);
    screen
  }

  fn load(&mut self, cx: &mut Context<'_, Self>) {
    let (tx, rx) = oneshot::channel();
    cx.global::<CommandBus>().send(Command::ListSessions(tx));
    cx.spawn(async move |this, cx| {
      if let Ok(sessions) = rx.await
        && let Ok(()) = this.update(cx, |screen, cx| screen.populate(sessions, cx))
      {}
    })
    .detach();
  }

  fn populate(&mut self, sessions: Vec<SessionV1>, cx: &mut Context<'_, Self>) {
    self.sessions = Some(sessions);
    cx.notify();
  }
}

impl Render for HomeScreen {
  fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    let theme = cx.global::<Theme>();

    let sessions_ui: AnyElement = match &self.sessions {
      None => div()
        .child(text!("Loading ..."))
        .with_animation(
          "sessions-loading",
          Animation::new(Duration::from_secs(1))
            .repeat()
            .with_easing(pulsating_between(0.2, 1.0)),
          |loading, delta| loading.opacity(delta),
        )
        .into_any_element(),
      Some(sessions) if sessions.is_empty() => div()
        .child(text!("No sessions yet."))
        .into_any_element(),
      Some(sessions) => div()
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

    screen_root()
      .child(top_bar()
        .child(title(text!("NovelCraft")))
        .child(settings_gear()))
      .child(content()
        .child(create_vignette(theme))
        .child(subtitle(text!("Sessions")))
        .child(sessions_ui))
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
