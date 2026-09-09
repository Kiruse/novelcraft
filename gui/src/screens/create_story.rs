use gpui::{Entity, Render, Window, text};
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
  }
}
