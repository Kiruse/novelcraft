use gpui::{Action, AnyElement, Div, Render, Task, Window, div, px, text};
use gpui::prelude::*;
use novelcraft_engine::game::session::SessionV1;

use super::screen;
use crate::actions::PlayStory;
use crate::comp::*;
use crate::theme::Theme;
use crate::{EngineQuery, StoryId};

pub(crate) struct StoryOverviewScreen {
  pub id: StoryId,
  /// Keeps the query watcher alive — cancelled when the screen drops.
  #[allow(dead_code)]
  task: Task<()>,
}

impl StoryOverviewScreen {
  pub fn create(cx: &mut Context<'_, Self>) -> Self {
    // Marked unchanged — the watcher fires on the refresh triggered by `enter`.
    let mut rx_sessions = cx.global::<EngineQuery<Vec<SessionV1>>>().rx();

    let task = cx.spawn(async move |this, cx| {
      while let Ok(()) = rx_sessions.changed().await {
        cx.update(|cx| {
          cx.notify(this.entity_id());
        });
      }
    });

    Self { id: StoryId::default(), task }
  }

  pub(crate) fn enter(&mut self, id: StoryId, cx: &mut Context<'_, Self>) {
    self.id = id;
    // The sessions cache may not know a freshly created session yet —
    // refresh it before resolving this story's details.
    cx.global::<EngineQuery<Vec<SessionV1>>>().refresh(cx);
    cx.notify();
  }
}

impl Render for StoryOverviewScreen {
  fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    let theme = cx.global::<Theme>();

    let result = cx.global::<EngineQuery<Vec<SessionV1>>>().curr();
    let details_ui: AnyElement = match result
      .value()
      .and_then(|sessions| sessions.iter().find(|session| session.id == self.id.0))
    {
      None => loading_text("session-loading", "Loading ...").into_any_element(),
      Some(session) => div()
        .flex()
        .flex_col()
        .gap_1()
        .w_full()
        .child(div().text_2xl().child(text!(session.title.clone())))
        .child(div().child(text!(session.exposition.clone())))
        .into_any_element(),
    };

    screen(text!("Story Overview")).closable()
      .child(banner_placeholder(&theme))
      .child(details_ui)
      .child(button("btn-play", text!("Play"))
        .primary(&theme)
        .into_element()
        .on_click({
          let id = self.id.clone();
          move |_, window, cx| window.dispatch_action(PlayStory(id.clone()).boxed_clone(), cx)
        }))
  }
}

/// Stand-in for the story's cover image banner.
fn banner_placeholder(theme: &Theme) -> Div {
  div()
    .w_full()
    .h(px(220.))
    .flex()
    .items_center()
    .justify_center()
    .border_1()
    .border_color(theme.border)
    .rounded_md()
    .bg(theme.text.alpha(0.05))
    .text_color(theme.label)
    .child(text!("No cover image"))
}
