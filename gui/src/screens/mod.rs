mod create_story;
mod home;
mod settings;
mod story_gameplay;
mod story_overview;

pub(crate) use create_story::CreateStoryScreen;
use gpui::{AnyElement, Div, IntoElement, ParentElement, Text};
pub(crate) use home::HomeScreen;
pub(crate) use settings::SettingsScreen;
pub(crate) use story_gameplay::StoryGameplayScreen;
pub(crate) use story_overview::StoryOverviewScreen;

use crate::{StoryId, comp::{btn_icon_close, content, screen_root, settings_gear, title, top_bar}};

#[derive(Debug, Clone, Default)]
pub enum Screen {
  #[default]
  Home,
  Settings,
  CreateStory,
  StoryOverview(StoryId),
  StoryGameplay(StoryId),
}

pub(crate) struct ScreenBase {
  closable: bool,
  title: Text,
  content: Vec<AnyElement>,
}

impl ScreenBase {
  #[inline(always)]
  pub fn closable(self) -> Self {
    Self {
      closable: true,
      ..self
    }
  }
}

impl IntoElement for ScreenBase {
  type Element = Div;
  fn into_element(self) -> Self::Element {
    screen_root()
      .child(top_bar()
        .child(title(self.title))
        .child(if self.closable {
          btn_icon_close().into_any_element()
        } else {
          settings_gear().into_any_element()
        }))
      .child(content().children(self.content))
  }
}

impl ParentElement for ScreenBase {
  fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
    self.content.extend(elements);
  }
}

pub(crate) fn screen(title: Text) -> ScreenBase {
  ScreenBase {
    closable: false,
    title,
    content: vec![],
  }
}
