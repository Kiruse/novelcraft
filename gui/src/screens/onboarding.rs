use dioxus::prelude::*;
use novelcraft_engine::config::ModelPurpose;

use crate::comps::{Button, BtnVariant, ModelFields, ModelGroup, Screen};
use crate::engine;
use crate::state::{Query, CONFIG};
use crate::toast::{push_toast, Toast};

const TOTAL_STEPS: usize = 2;
const REPO_URL: &str = "https://github.com/kiruse/novelcraft";
const LINK_CLASS: &str = "text-nc-label hover:underline cursor-pointer";

#[component]
pub fn Onboarding() -> Element {
  let mut step = use_signal(|| 0usize);
  let mut dm = use_signal(ModelFields::default);
  let mut suggestions = use_signal(ModelFields::default);
  let mut summarizer = use_signal(ModelFields::default);

  // Follow the config query as it publishes — fresh installs arrive with
  // the recommended defaults, existing configs keep whatever the user had
  // already set up.
  use_effect(move || {
    let Query::Recent(config) = &*CONFIG.read() else { return };
    dm.set(ModelFields::from_config(&config.models[ModelPurpose::DungeonMaster]));
    suggestions.set(ModelFields::from_config(&config.models[ModelPurpose::Suggestions]));
    summarizer.set(ModelFields::from_config(&config.models[ModelPurpose::Summarizer]));
  });

  let finish = move |_| {
    let mut config = CONFIG.read().value().cloned().unwrap_or_default();
    dm.read().clone().write_to(ModelPurpose::DungeonMaster, &mut config);
    suggestions.read().clone().write_to(ModelPurpose::Suggestions, &mut config);
    summarizer.read().clone().write_to(ModelPurpose::Summarizer, &mut config);
    config.onboarded = true;

    spawn(async move {
      // Republishing the config flips `onboarded` in the App gate, which
      // mounts the router — onboarding exits itself on success.
      if let Err(err) = engine::save_config(config).await {
        error!("Failed to save config during onboarding: {err}");
        push_toast(Toast::error("Failed to save — please try again"));
      }
    });
  };

  let dots = rsx! {
    div { class: "flex flex-row gap-2",
      for i in 0..TOTAL_STEPS {
        div {
          key: "{i}",
          class: if i == *step.read() {
            "w-2 h-2 rounded-full bg-nc-text"
          } else {
            "w-2 h-2 rounded-full bg-nc-text/30"
          }
        }
      }
    }
  };

  rsx! {
    Screen { title: "Welcome to NovelCraft", plain: true,
      { dots }
      { match *step.read() {
          0 => rsx! {
            div { class: "flex flex-col gap-4 w-full",
              div { class: "flex flex-col gap-1",
                div { class: "text-lg", "Free & open source" }
                div { class: "text-nc-text/80", "NovelCraft is developed in the open. No account, no license, no strings attached." }
                a {
                  class: LINK_CLASS,
                  href: REPO_URL,
                  target: "_blank",
                  rel: "noopener noreferrer",
                  "github.com/kiruse/novelcraft"
                }
              }
              div { class: "flex flex-col gap-1",
                div { class: "text-lg", "Everything stays on your device" }
                div { class: "text-nc-text/80", "Stories, sessions, characters and settings are stored locally. Nothing is uploaded, tracked or synced." }
              }
              div { class: "flex flex-col gap-1",
                div { class: "text-lg", "Online features are opt-in" }
                div { class: "text-nc-text/80", "Apart from the AI endpoint you configure in the next step — and the optional community features — no network calls are made on their own." }
              }
            }
            Button { label: "Next", variant: BtnVariant::Primary, on_click: move |_| step += 1 }
          },
          _ => rsx! {
            div { class: "flex flex-col gap-1 w-full",
              div { class: "text-xl", "Configure your models" }
              div { class: "text-nc-text/80",
                "NovelCraft talks to OpenAI-compatible APIs. By default all three agents run on "
                a {
                  class: LINK_CLASS,
                  href: "https://docs.novelcraft.quest/inference#quills",
                  target: "_blank",
                  rel: "noopener noreferrer",
                  "NovelCraft's hosted inference"
                }
                ", authenticated with your NovelCraft account — log in from the title bar any time."
              }
              div { class: "text-nc-text/80",
                "Prefer your own infrastructure? Switch a model to API Key mode and point it at any "
                a {
                  class: LINK_CLASS,
                  href: "https://openrouter.ai/google/gemma-4-26b-a4b-it",
                  target: "_blank",
                  rel: "noopener noreferrer",
                  "OpenAI-compatible endpoint"
                }
                " — OpenRouter, a local llama.cpp server, you name it. Free models are available "
                a {
                  class: LINK_CLASS,
                  href: "https://openrouter.ai/free",
                  target: "_blank",
                  rel: "noopener noreferrer",
                  "under openrouter/free"
                }
                ", which of course come with rather low rate limits. The same connection serves all three agents; you can fine-tune them any time in Settings."
              }
              div { class: "text-nc-text/80",
                "API keys are stored only on this device and are sent exclusively to the endpoint you configure."
              }
            }
            ModelGroup { purpose: ModelPurpose::DungeonMaster, models: [dm, suggestions, summarizer] }
            ModelGroup { purpose: ModelPurpose::Suggestions, models: [dm, suggestions, summarizer] }
            ModelGroup { purpose: ModelPurpose::Summarizer, models: [dm, suggestions, summarizer] }
            div { class: "flex flex-row gap-2 w-full",
              Button { label: "Back", variant: BtnVariant::Secondary, on_click: move |_| step -= 1 }
              Button { label: "Start Playing", variant: BtnVariant::Primary, on_click: finish }
            }
          },
      } }
    }
  }
}
