use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};
use std::time::Duration;

use futures::future::LocalBoxFuture;
use gpui::{
  Action, App, AsyncApp, Div, ElementId, Entity, Global, KeyBinding, Rgba, Stateful, Window, WindowOptions, div, prelude::*, px, relative, text,
};
use gpui_platform::application;
use log::*;
use novelcraft_engine::config::NovelCraftConfig;
use novelcraft_engine::error::EngineError;
use novelcraft_engine::game::pages::PageV1;
use novelcraft_engine::game::profile::ProfileV1;
use novelcraft_engine::game::session::SessionV1;
use novelcraft_engine::{AgentMessageChunk, game::engine::NovelCraftEngine};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, oneshot, watch};

use crate::comp::root;
use crate::error::GuiError;
use crate::screens::*;
use crate::theme::{Theme, deserialize_theme_name, serialize_theme_name};
use crate::util::Loggable;

mod comp;
mod error;
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
}

impl AppRoot {
  fn on_back(&mut self, cx: &mut Context<Self>) {
    self.switch_screen(match &self.screen {
      Screen::StoryGameplay(id) => Screen::StoryOverview(id.clone()),
      _ => Screen::Home,
    }, cx);
  }

  fn switch_screen(&mut self, new_screen: Screen, cx: &mut Context<Self>) {
    match &self.screen {
      Screen::CreateStory =>
        self.screen_create_story.update(cx, CreateStoryScreen::exit),
      _ => {}
    }

    self.screen = new_screen;

    match &self.screen {
      Screen::Home =>
        self.screen_home.update(cx, HomeScreen::enter),
      Screen::CreateStory =>
        self.screen_create_story.update(cx, CreateStoryScreen::enter),
      Screen::StoryGameplay(id) => {
        let id = id.clone();
        self.screen_story_gameplay.update(cx, |screen, cx| screen.enter(id, cx));
      }
      _ => {}
    }
  }

  /// Navigate the gameplay screen's active page, when it is showing.
  fn gameplay_nav(&mut self, delta: isize, cx: &mut Context<Self>) {
    if matches!(self.screen, Screen::StoryGameplay(_)) {
      self.screen_story_gameplay.update(cx, |screen, cx| screen.navigate(delta, cx));
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
    let _ = self.0.blocking_send(cmd).warn();
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
  let (tx_cmds, rx_cmds) = mpsc::channel::<Command>(100);
  let (tx_events, mut rx_events) = mpsc::channel::<AppEvents>(100);

  env_logger::init();

  let gui_config = load_config()?;
  let engine_config = NovelCraftConfig::load_sync()?;
  let engine_config2 = engine_config.clone();

  let h_engine = std::thread::spawn(move || {
    let tx_events = tx_events;
    let rt = tokio::runtime::Runtime::new().expect("Failed to initialize tokio async runtime");

    rt.block_on(async move {
      let mut engine = NovelCraftEngine::new(engine_config);
      let mut rx_cmds = rx_cmds;

      while let Some(cmd) = rx_cmds.recv().await {
        match cmd {
          Command::SwitchProfile(profile_id) => {
            engine.set_active_profile(Some(profile_id.clone()));
            AppEvents::SwitchProfile.send(&tx_events);
            if let Err(e) = engine.save().await {
              error!("Failed to persist active profile: {e}");
              Toast::error("Failed to save active profile")
                .send(&tx_events).await;
            }
          }
          Command::SaveConfig(config) => {
            match config.save().await.error() {
              Ok(_) => {
                engine.set_config(config);
                AppEvents::UpdateConfig.send(&tx_events);
              }
              Err(_) => {
                Toast::error("Failed to save new config").send(&tx_events).await;
              }
            }
          }
          Command::CreateSession { title, exposition } => {
            match engine.create_session(title, exposition).await {
              Ok(session) => {
                engine.set_session(Some(session));
                AppEvents::SwitchSession.send(&tx_events);
              }
              Err(err) => {
                engine.set_session(None);
                error!("Error creating session: {err}");
                Toast::error("Session creation failed").send(&tx_events).await;
              }
            }
          }
          Command::PlaySession { id } => {
            match SessionV1::load(id.0, &engine.config().profiles).await {
              Ok(session) => {
                engine.set_session(Some(session.clone()));
                AppEvents::SwitchSession.send(&tx_events);
              }
              Err(err) => {
                engine.set_session(None);
                error!("Error loading session: {err}");
                Toast::error("Failed to load session").send(&tx_events).await;
              }
            }
            let _ = tx_events.send(AppEvents::SwitchSession);
          }
          Command::GamePrompt { content, from_page, chunks, done } => {
            let _ = done.send(async {
              if let Some(page) = from_page {
                engine.fork(page).await?;
              }
              engine.prompt(content, chunks).await
            }.await);
          }
          Command::QueryEngine(querier) => {
            querier(&engine).await;
          }
        }
      }
    });
  });

  application().run(|cx: &mut App| {
    text_input::init(cx);
    cx.bind_keys([
      KeyBinding::new("alt-left", actions::PagePrev, None),
      KeyBinding::new("alt-right", actions::PageNext, None),
    ]);
    cx.set_global(gui_config.theme);
    cx.set_global(CommandBus(tx_cmds));
    register_queries(cx, engine_config2);

    let root = cx.new(|cx| AppRoot {
      screen: Screen::Home,
      toasts: Vec::new(),
      next_toast_id: 0,
      screen_home: cx.new(|cx| HomeScreen::create(cx)),
      screen_settings: cx.new(|cx| SettingsScreen::create(cx)),
      screen_create_story: cx.new(|cx| CreateStoryScreen::create(cx)),
      screen_story_overview: cx.new(|cx| StoryOverviewScreen::create(cx)),
      screen_story_gameplay: cx.new(|cx| StoryGameplayScreen::create(cx)),
    });

    register_actions(&root, cx);

    cx.spawn(async move |cx| {
      while let Some(ev) = rx_events.recv().await {
        match ev {
          AppEvents::Toast(toast) => {
            cx.update(move |cx| toast.dispatch(cx));
          }
          AppEvents::UpdateConfig => {
            EngineQuery::<NovelCraftConfig>::update_async(cx);
          }
          AppEvents::SwitchProfile => {
            EngineQuery::<Profiles>::update_async(cx);
          }
          AppEvents::SwitchSession => {
            EngineQuery::<SessionV1>::update_async(cx);
          }
        }
      }
    }).detach();

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

fn register_queries(cx: &mut App, config: NovelCraftConfig) {
  let profiles = Profiles {
    profiles: config.profiles.clone(),
    active_profile: config.active_profile.clone(),
  };

  let mut q = EngineQuery::<NovelCraftConfig>::new(async |engine| {
    Ok(engine.config().clone())
  });
  q.init(config);
  cx.set_global(q);

  let mut q = EngineQuery::<Profiles>::new(async |engine| {
    let cfg = engine.config();
    Ok(Profiles {
      profiles: cfg.profiles.clone(),
      active_profile: cfg.active_profile.clone(),
    })
  });
  q.init(profiles);
  cx.set_global(q);

  // Immediately load list of sessions
  let q = EngineQuery::<Vec<SessionV1>>::new(async |_engine| {
    Ok(NovelCraftEngine::list_sessions().await?)
  });
  q.refresh(cx);
  cx.set_global(q);

  cx.set_global(EngineQuery::<SessionV1>::new(async |engine| {
    Ok(engine.session()?.clone())
  }));

  cx.set_global(PageQuery::new());
}

fn register_actions(root: &Entity<AppRoot>, cx: &mut App) {
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
    move |_: &actions::PagePrev, cx| root.update(cx, |root, cx| root.gameplay_nav(-1, cx))
  });
  cx.on_action({
    let root = root.clone();
    move |_: &actions::PageNext, cx| root.update(cx, |root, cx| root.gameplay_nav(1, cx))
  });
  cx.on_action({
    let root = root.clone();
    move |ev: &actions::SpawnToast, cx| {
      root.update(cx, |root, cx| root.spawn_toast(ev.0.clone(), cx));
    }
  });
}

pub(crate) enum Command {
  /// Switch profile to the one with the given ID.
  SwitchProfile(String),
  /// Sync the config with the engine and persist it to disk.
  SaveConfig(NovelCraftConfig),
  /// Create a new session with the given title & exposition, replying with
  /// the created session (or None on failure).
  CreateSession {
    title: String,
    exposition: String,
  },
  /// Load the session with the given ID into the engine as the active
  /// session, replying with it (or None on failure).
  PlaySession { id: StoryId },
  /// Fork at the given page (if any) and submit a user prompt to the active
  /// session, streaming agent chunks; replies with the new page count
  /// (or None on failure).
  GamePrompt {
    content: String,
    from_page: Option<usize>,
    chunks: mpsc::Sender<AgentMessageChunk>,
    done: oneshot::Sender<Result<(), EngineError>>,
  },
  /// General purpose engine query. Primarily used internally with [EngineQuery].
  QueryEngine(Box<dyn for<'a> FnOnce(&'a NovelCraftEngine) -> LocalBoxFuture<'a, ()> + Send>),
}

impl Command {
  #[inline]
  pub fn dispatch(self, cx: &mut App) {
    let _ = cx.global::<CommandBus>().send(self);
  }
}

#[derive(Debug)]
enum AppEvents {
  Toast(Toast),
  UpdateConfig,
  SwitchSession,
  SwitchProfile,
}

impl AppEvents {
  #[inline(always)]
  fn send(self, tx: &mpsc::Sender<Self>) {
    let _ = tx.send(self);
  }
}

/// Dyn-compatible counterpart to [AsyncFn] queriers. The future returned by
/// an `async` closure is boxed here, at the abstraction boundary — [AsyncFn]
/// itself is not dyn compatible (GAT `CallRefFuture`).
trait EngineQuerier<T>: Send + Sync {
  fn query<'a>(
    &'a self,
    engine: &'a NovelCraftEngine,
  ) -> LocalBoxFuture<'a, Result<T, GuiError>>;
}

struct QuerierFn<F>(F);

impl<T, F> EngineQuerier<T> for QuerierFn<F>
where
  T: Send + Sync + 'static,
  F: for<'a> AsyncFn(&'a NovelCraftEngine) -> Result<T, GuiError> + Send + Sync + 'static,
{
  fn query<'a>(
    &'a self,
    engine: &'a NovelCraftEngine,
  ) -> LocalBoxFuture<'a, Result<T, GuiError>> {
    Box::pin((self.0)(engine))
  }
}

pub(crate) struct EngineQuery<T: Send + Sync + 'static> {
  tx: watch::Sender<EngineQueryResult<T>>,
  rx: watch::Receiver<EngineQueryResult<T>>,
  round: Arc<AtomicUsize>,
  querier: Arc<dyn EngineQuerier<T>>,
}

#[allow(dead_code)]
impl<T: Send + Sync + 'static> EngineQuery<T> {
  /// Create a query with the given querier closure. The closure runs on the
  /// engine thread with shared access to the [NovelCraftEngine]; its future
  /// borrows the engine and is therefore awaited there (no `Send` needed).
  /// Accepts `async` closures directly — boxing is handled internally.
  pub fn new<F>(querier: F) -> Self
  where
    F: for<'a> AsyncFn(&'a NovelCraftEngine) -> Result<T, GuiError>
      + Send
      + Sync
      + 'static,
  {
    let (tx, rx) = watch::channel(EngineQueryResult::default());
    Self {
      tx,
      rx,
      round: Arc::new(AtomicUsize::new(0)),
      querier: Arc::new(QuerierFn(querier)),
    }
  }

  /// Initialize the value of this engine query, marking it as successful.
  fn init(&mut self, value: T) {
    let round = self.round.load(AtomicOrdering::Acquire);
    let _ = self.tx.send(EngineQueryResult::Recent { value, round });
  }

  /// Reset the value & error of this query. Any in-flight refresh is
  /// discarded — its result will never be published.
  fn clear(&mut self) {
    self.round.fetch_add(1, AtomicOrdering::AcqRel);
    let _ = self.tx.send(EngineQueryResult::default());
  }

  /// Retrieve a pair of unchanged [watch::Receiver]s for both value & error.
  /// You may "listen" to `error.changed().await` to await the completion of
  /// the next [refresh][Self::refresh_with_bus].
  #[inline]
  pub fn rx(&self) -> watch::Receiver<EngineQueryResult<T>> {
    let mut rx_value = self.rx.clone();
    rx_value.mark_unchanged();
    rx_value
  }

  #[inline(always)]
  pub fn curr(&self) -> watch::Ref<'_, EngineQueryResult<T>> {
    self.rx.borrow()
  }

  #[inline]
  pub fn is_in_flight(&self) -> bool {
    let cmp = self.round.load(AtomicOrdering::Acquire);
    let last_round = self.rx.borrow().round();
    cmp != last_round
  }

  #[inline]
  pub fn is_stale(&self) -> bool {
    let val = &*self.rx.borrow();
    matches!(val, EngineQueryResult::Empty { .. }) || matches!(val, EngineQueryResult::Stale { .. })
  }

  #[inline(always)]
  pub fn has_error(&self) -> bool {
    match &*self.rx.borrow() {
      EngineQueryResult::Error { .. } |
      EngineQueryResult::Stale { .. } => true,
      _ => false,
    }
  }

  /// Run the querier on the engine thread, publishing the outcome to the
  /// watch channels. Guarded against multiple invocations with a round
  /// counter — only the newest invocation stores its result, though every
  /// invocation's returned receiver resolves once its query completed.
  ///
  /// The value `Receiver` will only be updated upon success, whereas the
  /// error `Receiver` will always be updated. If you need to await the
  /// completion of the refresh, you can follow this snippet:
  ///
  /// ```rust
  /// let (rx_value, mut rx_error) = query.refresh(bus);
  /// cx.spawn(|cx| {
  ///   let Ok(()) = rx_error.changed().await else { return };
  ///   // handle completion, e.g. call `cx.update(|cx| cx.notify(entity_id))`
  ///   // or retrieve current (potentially stale) `rx_value.borrow()`
  /// });
  /// ```
  pub fn refresh_with_bus(&self, command_bus: &CommandBus) -> watch::Receiver<EngineQueryResult<T>> {
    let counter = self.round.clone();
    let tx = self.tx.clone();
    // `fetch_add` yields the pre-increment value; this refresh's round is
    // the post-increment counter — matching `is_in_flight`'s expectation
    // that a published result's round equals the current counter.
    let round = counter.fetch_add(1, AtomicOrdering::AcqRel) + 1;
    let querier = self.querier.clone();
    command_bus.send(Command::QueryEngine(Box::new(
      move |engine: &NovelCraftEngine| {
        Box::pin(async move {
          let res = querier.query(engine).await;
          if counter.load(AtomicOrdering::Acquire) == round {
            match res {
              Ok(value) => {
                // Clear the error first — watchers observing both channels
                // must never pair a fresh value with a stale error.
                let _ = tx.send(EngineQueryResult::Recent { value, round });
              }
              Err(error) => {
                error!("EngineQuery<{}> error during refresh: {}", std::any::type_name::<T>(), error);
                tx.send_modify(|result| {
                  let prev = std::mem::replace(result, EngineQueryResult::Empty { round });
                  match prev {
                    EngineQueryResult::Recent { value, round } |
                    EngineQueryResult::Stale { value, round, .. } => {
                      *result = EngineQueryResult::Stale { value, error, round };
                    }
                    _ => {
                      *result = EngineQueryResult::Error { error, round };
                    }
                  }
                });
              }
            }
          }
        })
      },
    )));
    self.rx()
  }

  /// Convenience method for [refresh][Self::refresh_with_bus] that extracts the
  /// [CommandBus] from the app context.
  #[inline]
  pub fn refresh(&self, cx: &App) -> watch::Receiver<EngineQueryResult<T>> {
    let bus: &CommandBus = cx.global();
    self.refresh_with_bus(bus)
  }

  fn update_async(cx: &mut AsyncApp) {
    cx.update_global(|this: &mut Self, cx| {
      this.refresh(cx);
    });
  }
}

impl<T: Send + Sync + 'static> Global for EngineQuery<T> {}

#[derive(Debug)]
pub(crate) enum EngineQueryResult<T: Send + Sync + 'static> {
  Empty { round: usize },
  Error {
    error: GuiError,
    round: usize,
  },
  Stale {
    value: T,
    error: GuiError,
    round: usize,
  },
  Recent {
    value: T,
    round: usize,
  },
}

impl<T: Send + Sync + 'static> EngineQueryResult<T> {
  pub fn round(&self) -> usize {
    match self {
      Self::Empty { round } |
      Self::Error { round, .. } |
      Self::Recent { round, .. } |
      Self::Stale { round, .. } => *round
    }
  }

  pub fn value(&self) -> Option<&T> {
    match self {
      Self::Stale { value, .. } |
      Self::Recent { value, .. } => Some(value),
      _ => None,
    }
  }

  pub fn error(&self) -> Option<&GuiError> {
    match self {
      Self::Stale { error, .. } |
      Self::Error { error, .. } => Some(error),
      _ => None,
    }
  }

  #[inline]
  #[allow(unused)]
  pub fn is_empty(&self) -> bool {
    matches!(self, EngineQueryResult::Empty { .. })
  }

  #[inline]
  pub fn is_stale(&self) -> bool {
    matches!(self, Self::Stale { .. })
  }
}

impl<T: Send + Sync + 'static> Default for EngineQueryResult<T> {
  fn default() -> Self {
    Self::Empty { round: 0 }
  }
}

/// Specialized [EngineQuery] fetching a single page of the active session,
/// by index. The index is captured per [load][Self::load] call and read on
/// the engine thread, so a superseded in-flight load may query the newest
/// index — its result is discarded by the round guard regardless.
#[allow(dead_code)]
pub(crate) struct PageQuery {
  page_index: Arc<AtomicUsize>,
  inner: EngineQuery<PageV1>,
}

#[allow(dead_code)]
impl PageQuery {
  pub fn new() -> Self {
    let page_index = Arc::new(AtomicUsize::new(0));
    let index = page_index.clone();
    let inner = EngineQuery::<PageV1>::new(async move |engine| {
      let index = index.load(AtomicOrdering::Acquire);
      match engine.page(index).await {
        Some(page) => Ok(page),
        None => Err(EngineError::state(format!("page {index} not found")).into()),
      }
    });
    Self { page_index, inner }
  }

  /// Get an unchanged watcher for the currently active page to listen to
  /// in e.g. `cx.spawn(...)`.
  #[inline(always)]
  pub fn rx(&self) -> watch::Receiver<EngineQueryResult<PageV1>> {
    self.inner.rx()
  }

  /// Borrow the current page query result.
  #[inline(always)]
  pub fn curr(&self) -> watch::Ref<'_, EngineQueryResult<PageV1>> {
    self.inner.curr()
  }

  /// Fetch the page at `page_index` on the engine thread. Supersedes any
  /// in-flight load — only the newest invocation publishes its result.
  pub fn load(
    &self,
    page_index: usize,
    cx: &App,
  ) -> watch::Receiver<EngineQueryResult<PageV1>> {
    self.page_index.store(page_index, AtomicOrdering::Release);
    self.inner.refresh(cx)
  }

  /// Load the next page.
  #[inline(always)]
  pub fn load_next(&self, cx: &App) -> watch::Receiver<EngineQueryResult<PageV1>> {
    self.load(self.page_index() + 1, cx)
  }

  /// Load the previous page.
  #[inline(always)]
  pub fn load_prev(&self, cx: &App) -> watch::Receiver<EngineQueryResult<PageV1>> {
    self.load(self.page_index() - 1, cx)
  }

  /// Reset the value & error, discarding any in-flight load. Also rewinds
  /// the page index — a newly entered session starts at page 0.
  pub fn clear(&mut self) {
    self.page_index.store(0, AtomicOrdering::Release);
    self.inner.clear();
  }

  /// Get the current page index.
  #[inline]
  pub fn page_index(&self) -> usize {
    self.page_index.load(AtomicOrdering::Acquire)
  }
}

impl Global for PageQuery {}

pub struct Profiles {
  profiles: Vec<ProfileV1>,
  active_profile: Option<String>,
}

impl Profiles {
  #[inline(always)]
  pub fn iter(&self) -> impl Iterator<Item = &ProfileV1> {
    self.profiles.iter()
  }

  #[inline]
  pub fn active_profile(&self) -> Option<&ProfileV1> {
    let Some(active_profile) = &self.active_profile else { return None };
    self.profiles.iter().find(|p| p.id == *active_profile)
  }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Deserialize, JsonSchema)]
pub struct StoryId(pub String);

impl From<String> for StoryId {
  fn from(value: String) -> Self {
    Self(value)
  }
}

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

  #[inline(always)]
  async fn send(self, tx: &mpsc::Sender<AppEvents>) {
    let _ = tx.send(AppEvents::Toast(self)).await;
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

  actions!(nav, [Back, ShowSettings, CreateStory, PagePrev, PageNext]);

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
