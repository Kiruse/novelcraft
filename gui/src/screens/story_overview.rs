use gpui::{Render, Window, div};
use gpui::prelude::*;

use crate::StoryId;

pub(crate) struct StoryOverviewScreen {
  pub id: StoryId,
}

impl StoryOverviewScreen {
  pub fn create(_cx: &mut Context<'_, Self>) -> Self {
    Self { id: StoryId::default() }
  }
}

impl Render for StoryOverviewScreen {
  fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
    div()
  }
}
