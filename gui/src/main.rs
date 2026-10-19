mod api;
mod comps;
mod engine;
mod error;
mod markdown;
mod screens;
mod state;
mod titlebar;
mod toast;

use dioxus::prelude::*;
use log::warn;
use novelcraft_engine::config::NovelCraftConfig;

use crate::screens::Route;
use crate::titlebar::{ResizeHandles, TitleBar};

fn main() {
  env_logger::init();

  let config = match NovelCraftConfig::load_sync() {
    Ok(config) => config,
    Err(err) => {
      warn!("Failed to load engine config, using defaults: {err}");
      NovelCraftConfig::default()
    }
  };

  engine::init_engine(config.clone());
  state::BOOT_CONFIG.set(config).ok();

  dioxus::LaunchBuilder::new()
    .with_cfg(
      dioxus::desktop::Config::new().with_window(
        dioxus::desktop::WindowBuilder::new()
          .with_title("NovelCraft")
          .with_inner_size(dioxus::desktop::LogicalSize::new(1100., 800.))
          .with_decorations(false),
      ),
    )
    .launch(App);
}

#[component]
fn App() -> Element {
  rsx! {
    document::Stylesheet { href: asset!("/assets/tailwind.css") }
    div { class: "flex flex-col w-full h-full",
      TitleBar {}
      div { class: "flex-1 min-h-0",
        Router::<Route> {}
      }
    }
    ResizeHandles {}
    toast::Toasts {}
  }
}
