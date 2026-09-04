use gpui::*;
use gpui_platform::application;
use log::*;
use novelcraft_engine::config::NovelCraftConfig;
use novelcraft_engine::{AgentMessageChunk, game::engine::NovelCraftEngine};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use crate::comp::root;
use crate::screens::*;
use crate::theme::{Theme, deserialize_theme_name, serialize_theme_name};
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
  fn on_back(&mut self) {
    self.screen = match std::mem::take(&mut self.screen) {
      Screen::StoryGameplay(id) => Screen::StoryOverview(id),
      _ => Screen::Home,
    };
  }
}

impl Render for AppRoot {
  fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    let theme: &Theme = cx.global();
    let mut res = root(&theme);
    match &self.screen {
      Screen::Home => res = res.child(self.screen_home.clone()),
      Screen::Settings => res = res.child(self.screen_settings.clone()),
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

fn on_screen_action<A: Action>(
  cx: &mut App,
  root: &Entity<AppRoot>,
  to: impl Fn(&A) -> Screen + 'static,
) {
  let root = root.clone();
  cx.on_action(move |action: &A, cx| {
    let screen = to(action);
    root.update(cx, |root, _cx| root.screen = screen);
  });
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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct Config {
  #[serde(
    serialize_with = "serialize_theme_name",
    deserialize_with = "deserialize_theme_name"
  )]
  theme: Theme,
}

fn main() -> anyhow::Result<()> {
  let (tx_chunks, rx_chunks) = mpsc::channel::<AgentMessageChunk>(100);
  let (tx_cmds, rx_cmds) = mpsc::channel::<Command>(100);

  env_logger::init();

  let h_engine = std::thread::spawn(move || {
    let rt = tokio::runtime::Runtime::new().expect("Failed to initialize tokio async runtime");

    rt.block_on(async move {
      let mut engine = NovelCraftEngine::new(NovelCraftConfig::default());
      let mut rx_cmds = rx_cmds;

      while let Some(cmd) = rx_cmds.recv().await {
        match cmd {
          Command::SwitchProfile(profile_id) => {
            engine.set_active_profile(Some(profile_id));
          }
          Command::Prompt(prompt) => {
            engine.prompt(prompt, tx_chunks.clone()).await.warn();
          }
        }
      }
    });
  });

  let config = load_config()?;

  application().run(|cx: &mut App| {
    cx.set_global(config.theme);
    cx.set_global(CommandBus(tx_cmds));

    let root = cx.new(|cx| AppRoot {
      screen: Screen::Home,
      rx_chunks,
      screen_home: cx.new(|cx| HomeScreen::create(cx)),
      screen_settings: cx.new(|cx| SettingsScreen::create(cx)),
      screen_create_story: cx.new(|cx| CreateStoryScreen::create(cx)),
      screen_story_overview: cx.new(|cx| StoryOverviewScreen::create(cx)),
      screen_story_gameplay: cx.new(|cx| StoryGameplayScreen::create(cx)),
    });

    cx.on_action({
      let root = root.clone();
      move |_: &actions::Back, cx| root.update(cx, |root, _cx| root.on_back())
    });
    on_screen_action(cx, &root, |_: &actions::ShowSettings| Screen::Settings);
    on_screen_action(cx, &root, |_: &actions::CreateStory| Screen::CreateStory);
    on_screen_action(cx, &root, |ev: &actions::ShowStory| {
      Screen::StoryOverview(ev.0.clone())
    });
    on_screen_action(cx, &root, |ev: &actions::PlayStory| {
      Screen::StoryGameplay(ev.0.clone())
    });

    cx.open_window(WindowOptions::default(), {
      let root = root.clone();
      move |_, _| root.clone()
    })
    .unwrap();
    cx.activate(true);
  });

  let _ = h_engine.join();
  Ok(())
}

fn load_config() -> anyhow::Result<Config> {
  let path = dirs::config_dir().expect("Failed to read OS-specific config directory");
  let path = path.join("NovelCraft").join("gui.config.json");
  if !path.parent().unwrap().exists() {
    std::fs::create_dir_all(&path)?;
  }

  match std::fs::read_to_string(path) {
    Ok(config) => Ok(serde_json::from_str(&config)?),
    Err(e) => {
      warn!("Failed to read config file: {e}");
      Ok(Config::default())
    }
  }
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

  actions!(nav, [Back, ShowSettings, CreateStory,]);

  #[derive(Debug, Clone, PartialEq, Deserialize, JsonSchema, Action)]
  #[action(namespace = nav)]
  pub struct ShowStory(pub StoryId);

  #[derive(Debug, Clone, PartialEq, Deserialize, JsonSchema, Action)]
  #[action(namespace = nav)]
  pub struct PlayStory(pub StoryId);
}
