use gpui::{Entity, InteractiveElement, Render, Window, div, text};
use gpui::prelude::*;
use tokio::sync::oneshot;

use super::screen;
use crate::actions::ShowStory;
use crate::comp::*;
use crate::text_input::TextInput;
use crate::theme::Theme;
use crate::{Command, CommandBus, StoryId};

pub(crate) struct CreateStoryScreen {
  title: Entity<TextInput>,
  premise: Entity<TextInput>,
}

impl CreateStoryScreen {
  pub fn create(cx: &mut Context<'_, Self>) -> Self {
    Self {
      title: create_text_input(cx, false, "Title"),
      premise: create_text_input(
        cx,
        true,
        "Describe the premise of your story ..."
      ),
    }
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
          None => todo!("show error toast"),
        }
      }
    }).detach();
  }
}

impl Render for CreateStoryScreen {
  fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    let theme = cx.global::<Theme>();
    let (bg, fg) = (theme.bg, theme.text);

    screen(text!("Create Vignette")).closable()
      .child(field(&theme, "Title", &self.title))
      .child(field(&theme, "Premise", &self.premise))
      .child(div()
        .id("btn-create")
        .w_full()
        .p_2()
        .flex()
        .justify_center()
        .rounded_sm()
        .bg(fg)
        .text_color(bg)
        .cursor_pointer()
        .hover(|style| style.opacity(0.85))
        .on_click(cx.listener(|this, _, _, cx| this.submit(cx)))
        .child(text!("Create")))
  }
}
