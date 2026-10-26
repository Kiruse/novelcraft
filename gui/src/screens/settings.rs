use dioxus::prelude::*;
use novelcraft_engine::config::ModelPurpose;

use crate::comps::{Button, BtnVariant, Field, ModelFields, ModelGroup, Screen, INPUT_CLASS};
use crate::engine;
use crate::state::{Query, CONFIG};
use crate::toast::{push_toast, Toast};

#[component]
pub fn Settings() -> Element {
  let mut steps = use_signal(String::new);
  let mut system_prompt = use_signal(String::new);
  let mut threshold = use_signal(String::new);
  let mut dm = use_signal(ModelFields::default);
  let mut suggestions = use_signal(ModelFields::default);
  let mut summarizer = use_signal(ModelFields::default);
  let mut context_length = use_signal(String::new);

  // Populate the fields from the config query as soon as (and whenever)
  // it publishes a value.
  use_effect(move || {
    let Query::Recent(config) = &*CONFIG.read() else { return };
    steps.set(config.max_agent_steps.to_string());
    system_prompt.set(config.system_prompt.clone());
    threshold.set(config.summary_threshold.to_string());
    context_length.set(config.context_length.to_string());
    dm.set(ModelFields::from_config(&config.models[ModelPurpose::DungeonMaster]));
    suggestions.set(ModelFields::from_config(&config.models[ModelPurpose::Suggestions]));
    summarizer.set(ModelFields::from_config(&config.models[ModelPurpose::Summarizer]));
  });

  let save = move |_| {
    let mut config = CONFIG.read().value().cloned().unwrap_or_default();
    config.max_agent_steps = steps
      .read()
      .trim()
      .parse()
      .unwrap_or(config.max_agent_steps);
    config.summary_threshold = threshold
      .read()
      .trim()
      .parse()
      .unwrap_or(config.summary_threshold);
    config.context_length = context_length
      .read()
      .trim()
      .parse()
      .unwrap_or(config.context_length);
    config.system_prompt = system_prompt.read().clone();
    config.models[ModelPurpose::DungeonMaster] = dm.read().clone().into_config();
    config.models[ModelPurpose::Suggestions] = suggestions.read().clone().into_config();
    config.models[ModelPurpose::Summarizer] = summarizer.read().clone().into_config();

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

  // Flip the onboarding gate back on — saving republishes CONFIG, which
  // swaps the router for the onboarding screen in the App gate.
  let reset_onboarding = move |_| {
    let mut config = CONFIG.read().value().cloned().unwrap_or_default();
    config.onboarded = false;

    spawn(async move {
      if let Err(err) = engine::save_config(config).await {
        error!("Failed to reset onboarding: {err}");
        push_toast(Toast::error("Failed to reset onboarding"));
      }
    });
  };

  rsx! {
    Screen { title: "Settings", closable: true,
      Field { label: "Max Agent Steps",
        hint: "The agent may produce reasoning & game logic steps before generating story for you. This limits how many steps it may take to avoid an infinite loop.",
        input {
          class: INPUT_CLASS,
          value: "{steps}",
          oninput: move |ev: FormEvent| steps.set(ev.value()),
        }
      }
      Field { label: "Summary Threshold (turns)",
        hint: "The agent maintains a rolling summary of the recent story to improve recall on an endless conversation.",
        input {
          class: INPUT_CLASS,
          value: "{threshold}",
          oninput: move |ev: FormEvent| threshold.set(ev.value()),
        }
      }
      Field { label: "Context Window (tokens)",
        hint: "LLMs can only fit a certain number of input + output tokens called context window. This value is dependent on model, but cannot be easily retrieved from servers, so it is only for your own estimate and has no effect on prompt construction.",
        input {
          class: INPUT_CLASS,
          value: "{context_length}",
          oninput: move |ev: FormEvent| context_length.set(ev.value()),
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
        ModelGroup { purpose: ModelPurpose::DungeonMaster, models: [dm, suggestions, summarizer] }
        ModelGroup { purpose: ModelPurpose::Suggestions, models: [dm, suggestions, summarizer] }
        ModelGroup { purpose: ModelPurpose::Summarizer, models: [dm, suggestions, summarizer] }
      }
      Button { label: "Save", variant: BtnVariant::Primary, on_click: save }
      Button { label: "Reset Onboarding", variant: BtnVariant::Secondary, on_click: reset_onboarding }
    }
  }
}
