use std::time::Duration;

use exhaustive_map::{ExhaustiveMap, FiniteExt};
use gpui::{
  Action, Animation, AnimationExt, AnyElement, Div, Entity, InteractiveElement, Render, Stateful,
  Window, div, pulsating_between, text,
};
use gpui::prelude::*;
use novelcraft_engine::config::{DEFAULT_HOST, ModelConfig, ModelPurpose, NovelCraftConfig};
use novelcraft_engine::game::session::SessionV1;
use tokio::sync::oneshot;

use crate::actions::{CreateStory, ShowStory};
use crate::comp::*;
use crate::text_input::TextInput;
use crate::theme::Theme;
use crate::{Command, CommandBus, StoryId};

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

pub(crate) struct HomeScreen {
  sessions: Option<Vec<SessionV1>>,
}

impl HomeScreen {
  pub fn create(cx: &mut Context<'_, Self>) -> Self {
    let mut screen = Self { sessions: None };
    screen.load(cx);
    screen
  }

  fn load(&mut self, cx: &mut Context<'_, Self>) {
    let (tx, rx) = oneshot::channel();
    cx.global::<CommandBus>().send(Command::ListSessions(tx));
    cx.spawn(async move |this, cx| {
      if let Ok(sessions) = rx.await
        && let Ok(()) = this.update(cx, |screen, cx| screen.populate(sessions, cx))
      {}
    })
    .detach();
  }

  fn populate(&mut self, sessions: Vec<SessionV1>, cx: &mut Context<'_, Self>) {
    self.sessions = Some(sessions);
    cx.notify();
  }
}

impl Render for HomeScreen {
  fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    let theme = cx.global::<Theme>();

    let sessions_ui: AnyElement = match &self.sessions {
      None => div()
        .child(text!("Loading ..."))
        .with_animation(
          "sessions-loading",
          Animation::new(Duration::from_secs(1))
            .repeat()
            .with_easing(pulsating_between(0.2, 1.0)),
          |loading, delta| loading.opacity(delta),
        )
        .into_any_element(),
      Some(sessions) if sessions.is_empty() => div()
        .child(text!("No sessions yet."))
        .into_any_element(),
      Some(sessions) => div()
        .flex()
        .flex_col()
        .gap_2()
        .w_full()
        .children(sessions.iter().map(|session| {
          let story_id = StoryId(session.id.clone());
          session_card(theme, session).on_click(cx.listener(move |_, _, window, cx| {
            window.dispatch_action(ShowStory(story_id.clone()).boxed_clone(), cx)
          }))
        }))
        .into_any_element(),
    };

    screen_root()
      .child(top_bar()
        .child(title(text!("NovelCraft")))
        .child(settings_gear()))
      .child(content()
        .child(create_vignette(theme))
        .child(subtitle(text!("Sessions")))
        .child(sessions_ui))
  }
}

fn create_vignette(theme: &Theme) -> Stateful<Div> {
  div()
    .id("create-vignette")
    .w_full()
    .p_6()
    .flex()
    .flex_col()
    .gap_2()
    .border_1()
    .border_color(theme.border)
    .rounded_md()
    .cursor_pointer()
    .hover(|style| style.border_color(theme.label))
    .on_click(|_, window, cx| window.dispatch_action(CreateStory.boxed_clone(), cx))
    .child(
      div()
        .flex()
        .items_center()
        .justify_center()
        .gap_2()
        .text_2xl()
        .child(text!("+"))
        .child(text!("Create new Vignette"))
    )
    .child(
      div()
        .text_color(theme.label)
        .child(text!("Vignettes are free-form stories that don't follow any template. They resemble most closely what you get from other story generator platforms."))
    )
}

fn session_card(theme: &Theme, session: &SessionV1) -> Stateful<Div> {
  div()
    .id(session.id.clone())
    .w_full()
    .p_3()
    .flex()
    .flex_col()
    .gap_1()
    .border_1()
    .border_color(theme.border)
    .rounded_sm()
    .cursor_pointer()
    .hover(|style| style.border_color(theme.label))
    .child(div().text_lg().child(text!(session.title.clone())))
    .child(
      div()
        .text_sm()
        .text_color(theme.label)
        .child(text!(session
          .updated_at
          .with_timezone(&chrono::Local)
          .format("%Y-%m-%d %H:%M")
          .to_string()))
    )
    .child(div().child(text!(session.exposition.clone())))
}

//-----------------------------
// Settings Screen
//-----------------------------

struct ModelFields {
  base_url: Entity<TextInput>,
  api_key: Entity<TextInput>,
  model: Entity<TextInput>,
}

pub(crate) struct SettingsScreen {
  config: Option<NovelCraftConfig>,
  max_agent_steps: Entity<TextInput>,
  system_prompt: Entity<TextInput>,
  models: ExhaustiveMap<ModelPurpose, ModelFields>,
}

impl SettingsScreen {
  pub fn create(cx: &mut Context<'_, Self>) -> Self {
    let mut screen = Self {
      config: None,
      max_agent_steps: create_text_input(cx, false, "10"),
      system_prompt: create_text_input(cx, true, ""),
      models: ExhaustiveMap::from_fn(|_| ModelFields {
        base_url: create_text_input(cx, false, DEFAULT_HOST),
        api_key: create_text_input(cx, false, ""),
        model: create_text_input(cx, false, ""),
      }),
    };
    screen.load(cx);
    screen
  }

  fn load(&mut self, cx: &mut Context<'_, Self>) {
    let (tx, rx) = oneshot::channel();
    cx.global::<CommandBus>().send(Command::LoadConfig(tx));
    cx.spawn(async move |this, cx| {
      if let Ok(config) = rx.await
        && let Ok(()) = this.update(cx, |screen, cx| screen.populate(config, cx))
      {}
    })
    .detach();
  }

  fn populate(&mut self, config: NovelCraftConfig, cx: &mut Context<'_, Self>) {
    self.max_agent_steps.update(cx, |input, cx| {
      input.set_value(config.max_agent_steps.to_string(), cx)
    });
    self.system_prompt.update(cx, |input, cx| {
      input.set_value(config.system_prompt.clone(), cx)
    });

    let iter = self.models.values_mut().zip(config.models.values());
    for (fields, model_config) in iter {
      let ModelConfig::OpenAi {
        base_url,
        api_key,
        model,
      } = model_config;
      fields.base_url.update(cx, |input, cx| input.set_value(base_url, cx));
      fields.api_key.update(cx, |input, cx| input.set_value(api_key, cx));
      fields.model.update(cx, |input, cx| input.set_value(model, cx));
    }
    self.config = Some(config);
    cx.notify();
  }

  fn save(&mut self, cx: &mut Context<'_, Self>) {
    let mut config = self.config.clone().unwrap_or_default();
    config.max_agent_steps = self
      .max_agent_steps
      .read(cx)
      .value()
      .trim()
      .parse()
      .unwrap_or(config.max_agent_steps);
    config.system_prompt = self.system_prompt.read(cx).value().to_string();

    for key in ModelPurpose::iter_all() {
      config.models[key] = Self::model_config(&self.models[key], cx);
    }

    cx.global::<CommandBus>()
      .send(Command::SaveConfig(config));
  }

  fn model_config(fields: &ModelFields, cx: &Context<'_, SettingsScreen>) -> ModelConfig {
    ModelConfig::OpenAi {
      base_url: fields.base_url.read(cx).value().trim().to_string(),
      api_key: fields.api_key.read(cx).value().to_string(),
      model: fields.model.read(cx).value().trim().to_string(),
    }
  }
}

impl Render for SettingsScreen {
  fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    let theme = cx.global::<Theme>();
    let (bg, fg) = (theme.bg, theme.text);

    screen_root()
      .child(top_bar()
        .child(title(text!("Settings")))
        .child(btn_icon_close()))
      .child(content()
        .child(field(&theme, "Max Agent Steps", &self.max_agent_steps))
        .child(field(&theme, "System Prompt", &self.system_prompt))
        .child(div()
          .flex()
          .flex_col()
          .gap_2()
          .w_full()
          .child(div().text_xl().child(text!("Models")))
          .children(
            self
              .models
              .iter()
              .map(|(purpose, fields)| model_group(&theme, purpose.as_str(), fields))))
        .child(div()
          .id("btn-save")
          .w_full()
          .p_2()
          .flex()
          .justify_center()
          .rounded_sm()
          .bg(fg)
          .text_color(bg)
          .cursor_pointer()
          .hover(|style| style.opacity(0.85))
          .on_click(cx.listener(|this, _, _, cx| this.save(cx)))
          .child(text!("Save")))
      )
  }
}

fn model_group(theme: &Theme, label: &str, fields: &ModelFields) -> Div {
  field_group(theme, label)
    .child(field(theme, "Base URL", &fields.base_url))
    .child(field(theme, "API Key", &fields.api_key))
    .child(field(theme, "Model", &fields.model))
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
