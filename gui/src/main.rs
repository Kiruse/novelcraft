use gpui::*;
use gpui_platform::application;
use novelcraft_engine::{AgentMessageChunk, game::engine::GameEngine};
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
  screen_gameplay: Entity<GameplayScreen>,
  screen_home: Entity<HomeScreen>,
  screen_settings: Entity<SettingsScreen>,
  screen_story: Entity<StoryScreen>,
  rx_chunks: mpsc::Receiver<AgentMessageChunk>,
}

impl Render for AppRoot {
  fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    let theme: &Theme = cx.global();
    let mut res = root(&theme);
    match self.screen {
      Screen::Gameplay => res = res.child(self.screen_gameplay.clone()),
      Screen::Home     => res = res.child(self.screen_home.clone()),
      Screen::Settings => res = res.child(self.screen_settings.clone()),
      Screen::Story    => res = res.child(self.screen_story.clone()),
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
    app.open_window(WindowOptions::default(), |_, app| app.new(|cx| {
      // TODO: read theme from user preferences
      cx.set_global(Theme::default());
      cx.set_global(CommandBus(tx_cmds));

      AppRoot {
        screen: Screen::Home,
        rx_chunks,
        screen_gameplay: cx.new(|cx| GameplayScreen::create(cx)),
        screen_home:     cx.new(|cx| HomeScreen::create(cx)),
        screen_settings: cx.new(|cx| SettingsScreen::create(cx)),
        screen_story:    cx.new(|cx| StoryScreen::create(cx)),
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

mod actions {
  use gpui::actions;

  actions!(
    nav,
    [
      Back,
      ShowSettings,
    ]
  );
}
