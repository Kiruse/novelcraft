use std::sync::atomic::{AtomicU64, Ordering};

use dioxus::prelude::*;
use tokio::time::Duration;

/// Toast notifications, rendered as a fixed overlay in the bottom-right
/// corner. Any async task may raise a toast via [push_toast].
#[derive(Debug, Clone, PartialEq)]
pub struct Toast {
  pub id: u64,
  pub variant: ToastVariant,
  pub title: Option<String>,
  pub msg: String,
  pub duration: Duration,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
#[allow(dead_code)]
pub enum ToastVariant {
  #[default]
  Info,
  Success,
  Warn,
  Error,
}

impl ToastVariant {
  /// Tailwind accent classes for this variant.
  fn classes(&self) -> &'static str {
    match self {
      Self::Info => "border-nc-text",
      Self::Success => "border-nc-success",
      Self::Warn => "border-nc-warn",
      Self::Error => "border-nc-dangerfg",
    }
  }

  fn as_title(&self) -> &'static str {
    match self {
      Self::Info => "Info",
      Self::Success => "Success",
      Self::Warn => "Caution",
      Self::Error => "Error",
    }
  }
}

impl Toast {
  fn new(variant: ToastVariant, msg: impl Into<String>) -> Self {
    Self {
      id: 0,
      variant,
      title: Some(variant.as_title().to_string()),
      msg: msg.into(),
      duration: Duration::from_secs(5),
    }
  }

  #[allow(dead_code)]
  pub fn info(msg: impl Into<String>) -> Self {
    Self::new(ToastVariant::Info, msg)
  }

  pub fn success(msg: impl Into<String>) -> Self {
    Self::new(ToastVariant::Success, msg)
  }

  #[allow(dead_code)]
  pub fn warn(msg: impl Into<String>) -> Self {
    Self::new(ToastVariant::Warn, msg)
  }

  pub fn error(msg: impl Into<String>) -> Self {
    Self::new(ToastVariant::Error, msg)
  }
}

static TOASTS: GlobalSignal<Vec<Toast>> = GlobalSignal::new(Vec::new);
static NEXT_ID: AtomicU64 = AtomicU64::new(1);

/// Raise a toast. Auto-dismisses after the toast's duration unless its
/// duration is zero.
pub fn push_toast(mut toast: Toast) {
  toast.id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
  TOASTS.write().push(toast.clone());

  if toast.duration != Duration::ZERO {
    spawn(async move {
      tokio::time::sleep(toast.duration).await;
      dismiss_toast(toast.id);
    });
  }
}

pub fn dismiss_toast(id: u64) {
  TOASTS.write().retain(|toast| toast.id != id);
}

/// Fixed bottom-right toast overlay. Renders nothing while no toasts
/// are active.
#[component]
pub fn Toasts() -> Element {
  let toasts = TOASTS.read();
  if toasts.is_empty() {
    return rsx! {};
  }

  rsx! {
    div {
      class: "fixed bottom-4 right-4 min-w-60 max-w-4/5 flex flex-col-reverse items-center gap-2 z-50",
      for toast in toasts.iter() {
        { let id = toast.id; rsx! {
          div {
            key: "{id}",
            class: "w-full max-w-160 flex flex-col gap-1 p-3 items-stretch border rounded-sm bg-nc-bg shadow-lg {toast.variant.classes()}",
            div {
              class: "relative flex w-full",
              div {
                class: "text-nc-label",
                { toast.title.clone().unwrap_or_else(|| toast.variant.as_title().to_string()) }
              }
              button {
                class: "absolute right-1 text-nc-text text-lg cursor-pointer hover:opacity-70",
                onclick: move |_| dismiss_toast(id),
                "\u{00d7}"
              }
            }
            div { { toast.msg.clone() } }
          }
        } }
      }
    }
  }
}
