pub mod create_story;
pub mod home;
pub mod settings;
pub mod story_gameplay;
pub mod story_overview;

use dioxus::prelude::*;

use crate::screens::create_story::CreateStory;
use crate::screens::home::Home;
use crate::screens::settings::Settings;
use crate::screens::story_gameplay::StoryGameplay;
use crate::screens::story_overview::StoryOverview;

#[derive(Clone, Routable, Debug, PartialEq)]
pub enum Route {
  #[route("/")]
  Home {},
  #[route("/settings")]
  Settings {},
  #[route("/create")]
  CreateStory {},
  #[route("/story/:id")]
  StoryOverview { id: String },
  #[route("/play/:id")]
  StoryGameplay { id: String },
}

/// Navigate back: gameplay returns to its story's overview, everything
/// else returns home.
pub fn go_back(from: Route) {
  let nav = navigator();
  match from {
    Route::StoryGameplay { id } => {
      nav.push(Route::StoryOverview { id });
    }
    _ => {
      nav.push(Route::Home {});
    }
  }
}
