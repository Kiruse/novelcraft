use gpui::{Context, InteractiveElement, IntoElement, ParentElement, Render, StatefulInteractiveElement, Styled, Window, div, text};

use crate::{StoryId, comp::settings_gear};

#[derive(Debug, Clone, Default)]
pub enum Screen {
  #[default]
  Home,
  Settings,
  CreateStory,
  StoryOverview(StoryId),
  StoryGameplay(StoryId),
}

//-----------------------------
// Home Screen
//-----------------------------

pub(crate) struct HomeScreen {}

impl HomeScreen {
  pub fn create(_cx: &mut Context<'_, Self>) -> Self {
    Self {}
  }
}

impl Render for HomeScreen {
  fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
    div()
      .flex()
      .flex_col()
      .items_center()
      .w_full()
      .h_full()
      .child(
        div()
          .relative()
          .w_full()
          .flex()
          .flex_row()
          .justify_center()
          .child(
            div()
              .text_3xl()
              .child(text!("NovelCraft")),
          )
          .child(settings_gear())
      )
  }
}

//-----------------------------
// Settings Screen
//-----------------------------

pub(crate) struct SettingsScreen {}

impl SettingsScreen {
  pub fn create(_cx: &mut Context<'_, Self>) -> Self {
    Self {}
  }
}

impl Render for SettingsScreen {
  fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
    div()
      .relative()
      .child(
        div()
          .text_2xl()
          .child(text!("NovelCraft Settings")),
      )
      .child(
        div()
          .id("close-settings")
          .absolute()
          .top_4()
          .right_4()
          .text_xl()
          .cursor_pointer()
          .hover(|style| style.opacity(0.7))
          .on_click(|_e, _w, _app| {
            todo!()
            // let bus: &CommandBus = app.global();
            // bus.send(Command::Back);
          })
          .child(text!("\u{00d7}")),
      )
  }
}

//-----------------------------
// Create Story Screen
//-----------------------------

pub(crate) struct CreateStoryScreen {}

impl CreateStoryScreen {
  pub fn create(_cx: &mut Context<'_, Self>) -> Self {
    Self {}
  }
}

impl Render for CreateStoryScreen {
  fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
    div()
  }
}

//-----------------------------
// Story Overview Screen
//-----------------------------

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

//-----------------------------
// Story Gameplay Screen
//-----------------------------

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
