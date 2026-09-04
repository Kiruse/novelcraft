use gpui::{Div, Entity, Hsla, InteractiveElement, Render, Rgba, Window, div, px, text};
use gpui::prelude::*;
use novelcraft_engine::config::{DEFAULT_HOST, ModelConfig, NovelCraftConfig};
use tokio::sync::oneshot;

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

pub(crate) struct HomeScreen {}

impl HomeScreen {
  pub fn create(_cx: &mut Context<'_, Self>) -> Self {
    Self {}
  }
}

impl Render for HomeScreen {
  fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
    screen_root()
      .child(
        top_bar()
          .child(title(text!("NovelCraft")))
          .child(settings_gear())
      )
  }
}

//-----------------------------
// Settings Screen
//-----------------------------

const MODEL_GROUPS: [&str; 2] = ["Dungeon Master", "Suggestions"];

struct ModelFields {
  base_url: Entity<TextInput>,
  api_key: Entity<TextInput>,
  model: Entity<TextInput>,
}

pub(crate) struct SettingsScreen {
  config: Option<NovelCraftConfig>,
  max_agent_steps: Entity<TextInput>,
  system_prompt: Entity<TextInput>,
  models: Vec<ModelFields>,
}

impl SettingsScreen {
  pub fn create(cx: &mut Context<'_, Self>) -> Self {
    let mut screen = Self {
      config: None,
      max_agent_steps: create_text_input(cx, false, "10"),
      system_prompt: create_text_input(cx, true, ""),
      models: (0..2)
        .map(|_| ModelFields {
          base_url: create_text_input(cx, false, DEFAULT_HOST),
          api_key: create_text_input(cx, false, ""),
          model: create_text_input(cx, false, ""),
        })
        .collect(),
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

    let iter = self.models.iter_mut().zip(config.models.iter());
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
    let [dungeon_master, suggestions] = self.models.as_mut_slice() else {
      return;
    };
    config.models.dungeon_master = Self::model_config(dungeon_master, cx);
    config.models.suggestions = Self::model_config(suggestions, cx);
    cx.global::<CommandBus>()
      .send(Command::SaveConfig(Box::new(config)));
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
              .zip(MODEL_GROUPS)
              .map(|(fields, group)| model_group(&theme, group, fields))))
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
