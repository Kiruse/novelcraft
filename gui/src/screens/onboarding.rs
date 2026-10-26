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
  // the recommended defaults (OpenRouter + Gemma), existing configs keep
  // whatever the user had already set up.
  use_effect(move || {
    let Query::Recent(config) = &*CONFIG.read() else { return };
    dm.set(ModelFields::from_config(&config.models[ModelPurpose::DungeonMaster]));
    suggestions.set(ModelFields::from_config(&config.models[ModelPurpose::Suggestions]));
    summarizer.set(ModelFields::from_config(&config.models[ModelPurpose::Summarizer]));
  });

  let finish = move |_| {
    let mut config = CONFIG.read().value().cloned().unwrap_or_default();
    config.models[ModelPurpose::DungeonMaster] = dm.read().clone().into_config();
    config.models[ModelPurpose::Suggestions] = suggestions.read().clone().into_config();
    config.models[ModelPurpose::Summarizer] = summarizer.read().clone().into_config();
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
                div { class: "text-nc-text/80", "The app never calls out on its own — the only network use is the AI endpoint you connect in the next step." }
              }
            }
            Button { label: "Next", variant: BtnVariant::Primary, on_click: move |_| step += 1 }
          },
          _ => rsx! {
            div { class: "flex flex-col gap-1 w-full",
              div { class: "text-xl", "Bring your own model" }
              div { class: "text-nc-text/80",
                "NovelCraft talks to any OpenAI-compatible API. We recommend "
                a {
                  class: LINK_CLASS,
                  href: "https://openrouter.ai/google/gemma-4-26b-a4b-it",
                  target: "_blank",
                  rel: "noopener noreferrer",
                  "google/gemma-4-26b-a4b-it on OpenRouter"
                }
                " — create an API key at openrouter.ai/keys and paste it below."
              }
              div { class: "text-nc-text/80",
                "Free models are also available "
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
                "Your API key is stored only on this device and is sent exclusively to the endpoint you configure."
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
