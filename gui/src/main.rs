use std::time::Duration;

use gpui::{
  Action, App, Div, ElementId, Entity, Global, Rgba, Stateful, Window, WindowOptions, div, prelude::*, px, relative, text,
};
use gpui_platform::application;
use log::*;
use novelcraft_engine::config::NovelCraftConfig;
use novelcraft_engine::game::session::SessionV1;
use novelcraft_engine::{AgentMessageChunk, game::engine::NovelCraftEngine};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, oneshot};

use crate::comp::root;
use crate::screens::*;
use crate::theme::{Theme, deserialize_theme_name, serialize_theme_name};
use crate::util::{ExpectLoggable, Loggable};

mod comp;
mod screens;
mod text_input;
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
  toasts: Vec<(usize, Toast)>,
  next_toast_id: usize,
  rx_chunks: mpsc::Receiver<AgentMessageChunk>,
}

impl AppRoot {
  fn on_back(&mut self, cx: &mut Context<Self>) {
    self.switch_screen(match &self.screen {
      Screen::StoryGameplay(id) => Screen::StoryOverview(id.clone()),
      _ => Screen::Home,
    }, cx);
  }

  fn switch_screen(&mut self, new_screen: Screen, cx: &mut Context<Self>) {
    match self.screen {
      Screen::CreateStory =>
        self.screen_create_story.update(cx, CreateStoryScreen::exit),
      _ => {}
    }

    self.screen = new_screen;

    match self.screen {
      Screen::CreateStory =>
        self.screen_create_story.update(cx, CreateStoryScreen::enter),
      _ => {}
    }
  }

  fn spawn_toast(&mut self, toast: Toast, cx: &mut Context<Self>) {
    let id = self.next_toast_id;
    self.next_toast_id += 1;
    self.toasts.push((id, toast.clone()));
    cx.notify();

    if toast.duration != Duration::ZERO {
      let duration = toast.duration;
      cx.spawn(async move |this, cx| {
        cx.background_executor().timer(duration).await;
        this.update(cx, |root, cx| root.dismiss_toast(id, cx)).ok();
      })
      .detach();
    }
  }

  fn dismiss_toast(&mut self, id: usize, cx: &mut Context<Self>) {
    self.toasts.retain(|(toast_id, _)| *toast_id != id);
    cx.notify();
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
    if !self.toasts.is_empty() {
      let theme: &Theme = cx.global();
      res = res.child(
        div()
          .absolute()
          .bottom_4()
          .right_4()
          .min_w(px(240.))
          .max_w(relative(0.8))
          .flex()
          .flex_col_reverse()
          .items_center()
          .gap_2()
          .children(
            self
              .toasts
              .iter()
              .map(|(id, toast)| toast.to_elem(theme, *id, cx)),
          ),
      );
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
    root.update(cx, |root, cx| root.switch_screen(screen, cx));
  });
}

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
          Command::LoadConfig(tx) => {
            let config = NovelCraftConfig::load().await.expect_warn();
            engine.set_config(config.clone());
            let _ = tx.send(config);
          }
          Command::SaveConfig(config) => {
            config.save().await.error();
            engine.set_config(config);
          }
          Command::ListSessions(tx) => {
            let sessions = NovelCraftEngine::list_sessions()
              .await
              .expect_warn();
            let _ = tx.send(sessions);
          }
          Command::CreateSession { title, exposition, reply } => {
            let session = engine.create_session(title, exposition).await;
            session.error();
            let _ = reply.send(session.ok());
          }
        }
      }
    });
  });

  let config = load_config()?;

  application().run(|cx: &mut App| {
    text_input::init(cx);
    cx.set_global(config.theme);
    cx.set_global(CommandBus(tx_cmds));

    let root = cx.new(|cx| AppRoot {
      screen: Screen::Home,
      rx_chunks,
      toasts: Vec::new(),
      next_toast_id: 0,
      screen_home: cx.new(|cx| HomeScreen::create(cx)),
      screen_settings: cx.new(|cx| SettingsScreen::create(cx)),
      screen_create_story: cx.new(|cx| CreateStoryScreen::create(cx)),
      screen_story_overview: cx.new(|cx| StoryOverviewScreen::create(cx)),
      screen_story_gameplay: cx.new(|cx| StoryGameplayScreen::create(cx)),
    });

    cx.on_action({
      let root = root.clone();
      move |_: &actions::Back, cx| root.update(cx, |root, cx| root.on_back(cx))
    });
    on_screen_action(cx, &root, |_: &actions::ShowSettings| Screen::Settings);
    on_screen_action(cx, &root, |_: &actions::CreateStory| Screen::CreateStory);
    on_screen_action(cx, &root, |ev: &actions::ShowStory| {
      Screen::StoryOverview(ev.0.clone())
    });
    on_screen_action(cx, &root, |ev: &actions::PlayStory| {
      Screen::StoryGameplay(ev.0.clone())
    });
    cx.on_action({
      let root = root.clone();
      move |ev: &actions::SpawnToast, cx| {
        root.update(cx, |root, cx| root.spawn_toast(ev.0.clone(), cx));
      }
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

pub(crate) enum Command {
  /// Switch profile to the one with the given ID.
  SwitchProfile(String),
  /// Submit user prompt to the game engine.
  Prompt(String),
  /// Load the config from disk, sync it with the engine and reply with it.
  LoadConfig(oneshot::Sender<NovelCraftConfig>),
  /// Sync the config with the engine and persist it to disk.
  SaveConfig(NovelCraftConfig),
  /// List the player's saved sessions (metadata only), newest first.
  ListSessions(oneshot::Sender<Vec<SessionV1>>),
  /// Create a new session with the given title & exposition, replying with
  /// the created session (or None on failure).
  CreateSession {
    title: String,
    exposition: String,
    reply: oneshot::Sender<Option<SessionV1>>,
  },
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Deserialize, JsonSchema)]
pub struct StoryId(pub String);

#[derive(Debug, Clone, PartialEq, Deserialize, JsonSchema)]
pub struct Toast {
  pub variant: ToastVariant,
  pub msg: String,
  pub title: Option<String>,
  pub duration: Duration,
}

impl Toast {
  pub fn info(msg: impl Into<String>) -> Self {
    Self {
      variant: ToastVariant::Info,
      title: Some("Info".to_string()),
      msg: msg.into(),
      ..Self::default()
    }
  }

  pub fn success(msg: impl Into<String>) -> Self {
    Self {
      variant: ToastVariant::Success,
      title: Some("Success".to_string()),
      msg: msg.into(),
      ..Self::default()
    }
  }

  pub fn warn(msg: impl Into<String>) -> Self {
    Self {
      variant: ToastVariant::Warn,
      title: Some("Caution".to_string()),
      msg: msg.into(),
      ..Self::default()
    }
  }

  pub fn error(msg: impl Into<String>) -> Self {
    Self {
      variant: ToastVariant::Error,
      title: Some("Error".to_string()),
      msg: msg.into(),
      ..Self::default()
    }
  }

  pub fn variant_default(variant: ToastVariant, msg: impl Into<String>) -> Self {
    match variant {
      ToastVariant::Success => Self::success(msg),
      ToastVariant::Info    => Self::info(msg),
      ToastVariant::Warn    => Self::warn(msg),
      ToastVariant::Error   => Self::error(msg),
    }
  }

  #[inline(always)]
  pub fn to_spawn_action(self) -> actions::SpawnToast {
    actions::SpawnToast(self)
  }

  #[inline(always)]
  pub fn dispatch(self, cx: &mut App) {
    cx.defer(move |cx| cx.dispatch_action(&self.to_spawn_action()));
  }

  fn to_elem(&self, theme: &Theme, id: usize, cx: &Context<'_, AppRoot>) -> Stateful<Div> {
    let accent = self.variant.color(theme);
    let title = self.title
      .clone()
      .unwrap_or_else(|| self.variant.as_title().to_string());

    div()
      .id(ElementId::named_usize("toast", id))
      .w_full()
      .max_w(px(640.))
      .flex()
      .flex_col()
      .gap_1()
      .p_3()
      .items_stretch()
      .border_1()
      .border_color(accent)
      .rounded_sm()
      .bg(theme.bg)
      .shadow_lg()
      .child(div()
        .relative()
        .flex()
        .w_full()
        .child(div().text_color(accent).child(text!(title)))
        .child(div()
          .id(ElementId::named_usize("toast-cancel", id))
          .absolute()
          .right_1()
          .text_color(theme.text)
          .text_lg()
          .cursor_pointer()
          .hover(|style| style.opacity(0.7))
          .on_click(cx.listener(move |root, _, _, cx| root.dismiss_toast(id, cx)))
          .child(text!("\u{00d7}"))))
      .child(div().text_color(theme.text).child(text!(self.msg.clone())))
  }
}

impl Default for Toast {
  #[inline]
  fn default() -> Self {
    Self {
      variant: ToastVariant::Info,
      msg: String::new(),
      title: None,
      duration: Duration::from_secs(5),
    }
  }
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize, JsonSchema)]
pub enum ToastVariant {
  #[default]
  Info,
  Success,
  Warn,
  Error,
}

impl ToastVariant {
  /// Accent color for this variant.
  pub(crate) fn color(&self, theme: &Theme) -> Rgba {
    match self {
      Self::Info => theme.text,
      Self::Success => theme.success,
      Self::Warn => theme.warn,
      Self::Error => theme.danger_fg,
    }
  }

  fn as_title(&self) -> &'static str {
    match self {
      Self::Info    => "Info",
      Self::Success => "Success",
      Self::Warn    => "Caution",
      Self::Error   => "Error",
    }
  }
}

pub(crate) mod actions {
  use gpui::{Action, actions};
  use schemars::JsonSchema;
  use serde::Deserialize;

  use crate::{StoryId, Toast};

  actions!(nav, [Back, ShowSettings, CreateStory,]);

  #[derive(Debug, Clone, PartialEq, Deserialize, JsonSchema, Action)]
  #[action(namespace = nav)]
  pub struct ShowStory(pub StoryId);

  #[derive(Debug, Clone, PartialEq, Deserialize, JsonSchema, Action)]
  #[action(namespace = nav)]
  pub struct PlayStory(pub StoryId);

  #[derive(Debug, Clone, PartialEq, Deserialize, JsonSchema, Action)]
  #[action(namespace = toast)]
  pub struct SpawnToast(pub Toast);
}
