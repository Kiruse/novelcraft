use gpui::*;
use gpui_platform::application;
use novelcraft_engine::{AgentMessageChunk, game::engine::GameEngine};
use schemars::JsonSchema;
use serde::Deserialize;
use tokio::sync::mpsc;

use crate::comp::root;
use crate::screens::*;
use crate::theme::Theme;
use crate::util::Loggable;

mod comp;
mod screens;
mod theme;
mod util;

#[derive(Debug)]
struct AppRoot {
  screen: Screen,
  screen_home: Entity<HomeScreen>,
  screen_settings: Entity<SettingsScreen>,
  screen_create_story: Entity<CreateStoryScreen>,
  screen_story_overview: Entity<StoryOverviewScreen>,
  screen_story_gameplay: Entity<StoryGameplayScreen>,
  rx_chunks: mpsc::Receiver<AgentMessageChunk>,
}

impl AppRoot {
  fn on_back(&mut self, _: &actions::Back, _wnd: &mut Window, _cx: &mut Context<'_, Self>) {
    self.screen = match std::mem::take(&mut self.screen) {
      Screen::StoryGameplay(id) => Screen::StoryOverview(id),
      _ => Screen::Home,
    };
  }

  fn on_show_settings(&mut self, _: &actions::ShowSettings, _wnd: &mut Window, _cx: &mut Context<'_, Self>) {
    self.screen = Screen::Settings;
  }

  fn on_create_story(&mut self, _: &actions::CreateStory, _wnd: &mut Window, _cx: &mut Context<'_, Self>) {
    self.screen = Screen::CreateStory;
  }

  fn on_show_story(&mut self, ev: &actions::ShowStory, _wnd: &mut Window, _cx: &mut Context<'_, Self>) {
    self.screen = Screen::StoryOverview(ev.0.clone())
  }

  fn on_play_story(&mut self, ev: &actions::PlayStory, _wnd: &mut Window, _cx: &mut Context<'_, Self>) {
    self.screen = Screen::StoryGameplay(ev.0.clone())
  }
}

impl Render for AppRoot {
  fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    let theme: &Theme = cx.global();
    let mut res = root(&theme)
      .on_action(cx.listener(Self::on_back))
      .on_action(cx.listener(Self::on_show_settings))
      .on_action(cx.listener(Self::on_create_story))
      .on_action(cx.listener(Self::on_show_story))
      .on_action(cx.listener(Self::on_play_story));
    match &self.screen {
      Screen::Home        => res = res.child(self.screen_home.clone()),
      Screen::Settings    => res = res.child(self.screen_settings.clone()),
      Screen::CreateStory => res = res.child(self.screen_create_story.clone()),
      Screen::StoryOverview(id) => {
        cx.update_entity(&self.screen_story_overview, |screen, _cx| {
          screen.id = id.clone();
        });
        res = res.child(self.screen_story_overview.clone());
      }
      Screen::StoryGameplay(id) => {
        cx.update_entity(&self.screen_story_gameplay, |screen, _cx| {
          screen.id = id.clone();
        });
        res = res.child(self.screen_story_gameplay.clone());
      }
    }
    res
  }
}

#[derive(Debug)]
pub(crate) struct CommandBus(mpsc::Sender<Command>);

impl CommandBus {
  #[inline]
  pub fn send(&self, cmd: Command) {
    self.0.blocking_send(cmd).warn();
  }
}

impl Global for CommandBus {}

fn main() {
  let (tx_chunks, rx_chunks) = mpsc::channel::<AgentMessageChunk>(100);
  let (tx_cmds, rx_cmds) = mpsc::channel::<Command>(100);

  let h_engine = std::thread::spawn(move || {
    let rt = tokio::runtime::Runtime::new()
      .expect("Failed to initialize tokio async runtime");

    rt.block_on(async move {
      let mut engine = GameEngine::new();
      let mut rx_cmds = rx_cmds;

      while let Some(cmd) = rx_cmds.recv().await {
        match cmd {
          Command::SwitchProfile(profile_id) => {
            engine = engine.with_profile_id(profile_id);
          }
          Command::Prompt(prompt) => {
            engine.prompt(prompt, tx_chunks.clone()).await.warn();
          }
        }
      }
    });
  });

  application().run(|app: &mut App| {
    app.open_window(WindowOptions::default(), |_wnd, app| app.new(|cx| {
      // TODO: read theme from user preferences
      cx.set_global(Theme::default());
      cx.set_global(CommandBus(tx_cmds));

      AppRoot {
        screen: Screen::Home,
        rx_chunks,
        screen_home:           cx.new(|cx| HomeScreen::create(cx)),
        screen_settings:       cx.new(|cx| SettingsScreen::create(cx)),
        screen_create_story:   cx.new(|cx| CreateStoryScreen::create(cx)),
        screen_story_overview: cx.new(|cx| StoryOverviewScreen::create(cx)),
        screen_story_gameplay: cx.new(|cx| StoryGameplayScreen::create(cx)),
      }
    }))
    .unwrap();
    app.activate(true);
  });

  let _ = h_engine.join();
}

#[derive(Debug, Clone)]
enum Command {
  /// Switch profile to the one with the given ID.
  SwitchProfile(String),
  /// Submit user prompt to the game engine.
  Prompt(String),
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Deserialize, JsonSchema)]
pub struct StoryId(pub String);

mod actions {
  use gpui::{Action, actions};
  use schemars::JsonSchema;
  use serde::Deserialize;

  use crate::StoryId;

  actions!(
    nav,
    [
      Back,
      ShowSettings,
      CreateStory,
    ]
  );

  #[derive(Debug, Clone, PartialEq, Deserialize, JsonSchema, Action)]
  #[action(namespace = nav)]
  pub struct ShowStory(pub StoryId);

  #[derive(Debug, Clone, PartialEq, Deserialize, JsonSchema, Action)]
  #[action(namespace = nav)]
  pub struct PlayStory(pub StoryId);
}
