use gpui::{Render, Window, div};
use gpui::prelude::*;

use crate::StoryId;

pub(crate) struct StoryGameplayScreen {
  pub id: StoryId,
}

impl StoryGameplayScreen {
  pub fn create(_cx: &mut Context<'_, Self>) -> Self {
    Self { id: StoryId::default() }
  }
}

impl Render for StoryGameplayScreen {
  fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
    div()
  }
}
