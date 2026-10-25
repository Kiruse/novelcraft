mod api;
mod comps;
mod engine;
mod error;
mod markdown;
mod modules;
mod screens;
mod state;
mod time;
mod titlebar;
mod toast;

use dioxus::prelude::*;
use log::warn;
use novelcraft_engine::config::NovelCraftConfig;

use crate::screens::Route;
use crate::titlebar::{ResizeHandles, TitleBar};

fn main() {
  // env_logger has no console backend on wasm — logs would vanish. The
  // console_log crate forwards the `log` facade to the browser console.
  #[cfg(not(target_arch = "wasm32"))]
  env_logger::init();
  #[cfg(target_arch = "wasm32")]
  console_log::init_with_level(log::Level::Info).ok();

  // On wasm, panics abort the instance and otherwise surface in the
  // browser console as a bare "RuntimeError: unreachable" — the hook
  // logs the actual panic message & stack first.
  #[cfg(target_arch = "wasm32")]
  console_error_panic_hook::set_once();

  let builder = dioxus::LaunchBuilder::new();
  #[cfg(not(target_arch = "wasm32"))]
  let builder = builder.with_cfg(
    dioxus::desktop::Config::new().with_window(
      dioxus::desktop::WindowBuilder::new()
        .with_title("NovelCraft")
        .with_inner_size(dioxus::desktop::LogicalSize::new(1100., 800.))
        .with_decorations(false),
    ),
  );
  builder.launch(App);
}

#[component]
fn App() -> Element {
  // The engine boots inside the runtime on ALL targets: the config load is
  // async everywhere (tokio on desktop, OPFS on web), so nothing renders
  // until it completes.
  let mut booted = use_signal(|| false);

  use_effect(move || {
    spawn(async move {
      let config = match NovelCraftConfig::load().await {
        Ok(config) => config,
        Err(err) => {
          warn!("Failed to load engine config, using defaults: {err}");
          NovelCraftConfig::default()
        }
      };
      engine::init_engine(config.clone());
      state::BOOT_CONFIG.set(config).ok();
      engine::refresh_config();
      engine::refresh_profiles();
      engine::refresh_sessions();
      booted.set(true);
    });
  });

  if !*booted.read() {
    return rsx! {
      document::Stylesheet { href: asset!("/assets/tailwind.css") }
    };
  }

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
