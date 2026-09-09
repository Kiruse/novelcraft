mod create_story;
mod home;
mod settings;
mod story_gameplay;
mod story_overview;

pub(crate) use create_story::CreateStoryScreen;
pub(crate) use home::HomeScreen;
pub(crate) use settings::SettingsScreen;
pub(crate) use story_gameplay::StoryGameplayScreen;
pub(crate) use story_overview::StoryOverviewScreen;

use crate::StoryId;

#[derive(Debug, Clone, Default)]
pub enum Screen {
  #[default]
  Home,
  Settings,
  CreateStory,
  StoryOverview(StoryId),
  StoryGameplay(StoryId),
}
