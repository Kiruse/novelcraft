use exhaustive_map::{ExhaustiveMap, FiniteExt};
use gpui::{Div, Entity, InteractiveElement, Render, Window, div, text};
use gpui::prelude::*;
use novelcraft_engine::config::{DEFAULT_HOST, ModelConfig, ModelPurpose, NovelCraftConfig};
use tokio::sync::oneshot;

use crate::comp::*;
use crate::text_input::TextInput;
use crate::theme::Theme;
use crate::{Command, CommandBus};

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
