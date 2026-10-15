use dioxus::prelude::*;
use novelcraft_engine::config::{DEFAULT_HOST, ModelConfig, ModelPurpose};

use crate::comps::{Button, BtnVariant, Field, FieldGroup, Screen, INPUT_CLASS};
use crate::engine;
use crate::state::{Query, CONFIG};
use crate::toast::{push_toast, Toast};

/// Editable fields of a single model config.
#[derive(Debug, Clone, Default, PartialEq)]
struct ModelFields {
  base_url: String,
  api_key: String,
  model: String,
}

impl ModelFields {
  fn from_config(config: &ModelConfig) -> Self {
    let ModelConfig::OpenAi {
      base_url,
      api_key,
      model,
    } = config;
    Self {
      base_url: base_url.clone(),
      api_key: api_key.clone(),
      model: model.clone(),
    }
  }

  fn into_config(self) -> ModelConfig {
    ModelConfig::OpenAi {
      base_url: self.base_url.trim().to_string(),
      api_key: self.api_key.clone(),
      model: self.model.trim().to_string(),
    }
  }
}

#[component]
pub fn Settings() -> Element {
  let mut steps = use_signal(String::new);
  let mut system_prompt = use_signal(String::new);
  let mut dm = use_signal(ModelFields::default);
  let mut suggestions = use_signal(ModelFields::default);

  // Populate the fields from the config query as soon as (and whenever)
  // it publishes a value.
  use_effect(move || {
    let Query::Recent(config) = &*CONFIG.read() else { return };
    steps.set(config.max_agent_steps.to_string());
    system_prompt.set(config.system_prompt.clone());
    dm.set(ModelFields::from_config(&config.models[ModelPurpose::DungeonMaster]));
    suggestions.set(ModelFields::from_config(&config.models[ModelPurpose::Suggestions]));
  });

  let save = move |_| {
    let mut config = CONFIG.read().value().cloned().unwrap_or_default();
    config.max_agent_steps = steps
      .read()
      .trim()
      .parse()
      .unwrap_or(config.max_agent_steps);
    config.system_prompt = system_prompt.read().clone();
    config.models[ModelPurpose::DungeonMaster] = dm.read().clone().into_config();
    config.models[ModelPurpose::Suggestions] = suggestions.read().clone().into_config();

    spawn(async move {
      match engine::save_config(config).await {
        Ok(()) => push_toast(Toast::success("Settings saved!")),
        Err(err) => {
          error!("Failed to save settings: {err}");
          push_toast(Toast::error("Failed to save new config"));
        }
      }
    });
  };

  rsx! {
    Screen { title: "Settings", closable: true,
      Field { label: "Max Agent Steps",
        input {
          class: INPUT_CLASS,
          value: "{steps}",
          oninput: move |ev: FormEvent| steps.set(ev.value()),
        }
      }
      Field { label: "System Prompt",
        textarea {
          class: INPUT_CLASS,
          rows: 4,
          value: "{system_prompt}",
          oninput: move |ev: FormEvent| system_prompt.set(ev.value()),
        }
      }
      div { class: "flex flex-col gap-2 w-full",
        div { class: "text-xl", "Models" }
        ModelGroup { label: ModelPurpose::DungeonMaster.as_str(), fields: dm }
        ModelGroup { label: ModelPurpose::Suggestions.as_str(), fields: suggestions }
      }
      Button { label: "Save", variant: BtnVariant::Primary, on_click: save }
    }
  }
}

#[derive(Props, Clone, PartialEq)]
struct ModelGroupProps {
  label: String,
  fields: Signal<ModelFields>,
}

#[component]
fn ModelGroup(props: ModelGroupProps) -> Element {
  let mut fields = props.fields;

  let set_base_url = move |ev: FormEvent| {
    fields.write().base_url = ev.value();
  };
  let set_api_key = move |ev: FormEvent| {
    fields.write().api_key = ev.value();
  };
  let set_model = move |ev: FormEvent| {
    fields.write().model = ev.value();
  };

  rsx! {
    FieldGroup { label: props.label,
      Field { label: "Base URL",
        input {
          class: INPUT_CLASS,
          placeholder: DEFAULT_HOST,
          value: "{fields.read().base_url}",
          oninput: set_base_url,
        }
      }
      Field { label: "API Key",
        input {
          class: INPUT_CLASS,
          value: "{fields.read().api_key}",
          oninput: set_api_key,
        }
      }
      Field { label: "Model",
        input {
          class: INPUT_CLASS,
          value: "{fields.read().model}",
          oninput: set_model,
        }
      }
    }
  }
}
