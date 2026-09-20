use std::fmt::Display;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};

use gpui::{App, AppContext, Entity, Hsla, Task};
pub use log::Level as LogLevel;
use tokio::sync::oneshot;

use crate::{Toast, ToastVariant};

#[allow(unused)]
pub trait Loggable {
  fn log(self, level: LogLevel) -> Self;

  #[inline(always)]
  fn trace(self) -> Self where Self: Sized {
    self.log(LogLevel::Trace)
  }

  #[inline(always)]
  fn debug(self) -> Self where Self: Sized {
    self.log(LogLevel::Debug)
  }

  #[inline(always)]
  fn info(self) -> Self where Self: Sized {
    self.log(LogLevel::Info)
  }

  #[inline(always)]
  fn warn(self) -> Self where Self: Sized {
    self.log(LogLevel::Warn)
  }

  #[inline(always)]
  fn error(self) -> Self where Self: Sized {
    self.log(LogLevel::Error)
  }
}

impl<T, E: Display> Loggable for Result<T, E> {
  fn log(self, level: LogLevel) -> Self {
    match &self {
      Ok(_) => {}
      Err(e) => log::log!(level.into(), "Err: {e}"),
    }
    self
  }
}

/// Unwraps the value, but logs a message at specified warning
/// levels when invalid and returns default value instead.
#[allow(unused)]
pub trait ExpectLoggable<T> {
  fn expect_log(self, level: LogLevel) -> T;

  #[inline(always)]
  fn expect_trace(self) -> T where Self: Sized {
    self.expect_log(LogLevel::Trace)
  }

  #[inline(always)]
  fn expect_debug(self) -> T where Self: Sized {
    self.expect_log(LogLevel::Debug)
  }

  #[inline(always)]
  fn expect_info(self) -> T where Self: Sized {
    self.expect_log(LogLevel::Info)
  }

  #[inline(always)]
  fn expect_warn(self) -> T where Self: Sized {
    self.expect_log(LogLevel::Warn)
  }

  #[inline(always)]
  fn expect_error(self) -> T where Self: Sized {
    self.expect_log(LogLevel::Error)
  }
}

impl<T: Default, E: Display> ExpectLoggable<T> for Result<T, E> {
  fn expect_log(self, level: LogLevel) -> T {
    match self {
      Ok(v) => v,
      Err(e) => {
        log::log!(level.into(), "Err: {e}");
        Default::default()
      }
    }
  }
}

/// Dispatches toasts based on the outcome of a [`Result`].
///
/// The `cx` receiver is `&mut App` — [`gpui::Context`] derefs to it,
/// so view code can simply pass its own context.
#[allow(unused)]
pub trait Toastable {
  /// Dispatch a toast based on self's contents.
  fn report_toast(self, variant: ToastVariant, cx: &mut App) -> Self;
  /// Info toast (message from the value) on success, error toast on failure.
  fn info_toast(self, cx: &mut App) -> Self where Self: Sized {
    self.report_toast(ToastVariant::Info, cx)
  }
  /// Success toast (message from the value) on success, error toast on failure.
  fn success_toast(self, cx: &mut App) -> Self where Self: Sized {
    self.report_toast(ToastVariant::Success, cx)
  }
  /// Warning toast (message from the value) on success, error toast on failure.
  fn warn_toast(self, cx: &mut App) -> Self where Self: Sized {
    self.report_toast(ToastVariant::Warn, cx)
  }
  /// Error toast on failure, nothing on success.
  fn error_toast(self, cx: &mut App) -> Self where Self: Sized {
    self.report_toast(ToastVariant::Error, cx)
  }
}

impl<T, E: Display> Toastable for Result<T, E> {
  fn report_toast(self, variant: ToastVariant, cx: &mut App) -> Self {
    if let Err(e) = &self {
      Toast::variant_default(variant, format!("{e}")).dispatch(cx);
    }
    self
  }
}

#[allow(unused)]
pub trait HslaExt {
  fn hue(self, cb: impl FnOnce(f32) -> f32) -> Self;
  fn sat(self, cb: impl FnOnce(f32) -> f32) -> Self;
  fn lum(self, cb: impl FnOnce(f32) -> f32) -> Self;
  fn al(self, cb: impl FnOnce(f32) -> f32) -> Self;
}

impl HslaExt for Hsla {
  #[inline(always)]
  fn al(self, cb: impl FnOnce(f32) -> f32) -> Self {
    Self {
      a: cb(self.a).clamp(0., 1.),
      ..self
    }
  }

  #[inline(always)]
  fn hue(self, cb: impl FnOnce(f32) -> f32) -> Self {
    Self {
      h: cb(self.h).clamp(0., 1.),
      ..self
    }
  }

  #[inline(always)]
  fn lum(self, cb: impl FnOnce(f32) -> f32) -> Self {
    Self {
      l: cb(self.l).clamp(0., 1.),
      ..self
    }
  }

  #[inline(always)]
  fn sat(self, cb: impl FnOnce(f32) -> f32) -> Self {
    Self {
      s: cb(self.s).clamp(0., 1.),
      ..self
    }
  }
}

#[derive(Debug)]
#[allow(unused)]
pub struct Loader<T: Send> {
  round: Arc<AtomicUsize>,
  value: Entity<Option<T>>,
  task: Option<Task<()>>,
}

#[allow(unused)]
impl<T: Send + 'static> Loader<T> {
  #[inline(always)]
  pub fn new(cx: &mut App) -> Self {
    Self {
      round: Arc::new(AtomicUsize::new(0)),
      value: cx.new(|_| None),
      task: None,
    }
  }

  #[inline(always)]
  pub fn value<'a>(&self, cx: &'a App) -> &'a Option<T> {
    &self.value.read(cx)
  }

  #[inline(always)]
  pub fn round(&self) -> usize {
    self.round.load(AtomicOrdering::Acquire)
  }

  /// Load an async value, guarding against multiple invocations with a
  /// round counter. Only the last call will populate the value.
  pub fn load(
    &mut self,
    cx: &mut App,
    loader: impl AsyncFnOnce() -> anyhow::Result<T> + 'static,
  ) -> oneshot::Receiver<LoaderResult> {
    let round = self.round.clone();
    let this_round = round.fetch_add(1, AtomicOrdering::AcqRel);
    self.value.update(cx, |v, _| *v = None);
    let handle = self.value.clone();
    let (tx, rx) = oneshot::channel();
    self.task = Some(cx.spawn(async move |cx| {
      let res = loader().await.warn();
      let stored_round = round.load(AtomicOrdering::Acquire);
      match res {
        Ok(value) if stored_round == this_round => {
          handle.update(cx, |v, _| *v = Some(value));
          let _ = tx.send(LoaderResult::Success);
        }
        Err(err) => {
          handle.update(cx, |v, _| *v = None);
          let _ = tx.send(LoaderResult::Failure(err));
        }
        _ => {
          let _ = tx.send(LoaderResult::Stale);
        }
      }
    }));
    rx
  }

  /// Reset the state of this async loader back to initial.
  #[inline]
  pub fn reset(&mut self, cx: &mut App) {
    self.round.store(0, AtomicOrdering::Release);
    self.value.update(cx, |v, _| *v = None);
    self.task = None;
  }

  #[inline(always)]
  pub fn cancel(&mut self) {
    self.task = None;
  }

  #[inline(always)]
  pub fn is_pending(&self, cx: &App) -> bool {
    self.value.read(cx).is_none()
  }

  #[inline(always)]
  pub fn is_busy(&self, cx: &App) -> bool {
    self.task.is_some() && self.value.read(cx).is_none()
  }

  #[inline(always)]
  pub fn is_done(&self, cx: &App) -> bool {
    self.value.read(cx).is_some()
  }
}

#[derive(Debug, Default)]
#[allow(unused)]
pub enum LoaderResult {
  /// Emitted when the retrieval was successful & the value was updated.
  Success,
  /// Emitted when the retrieval was successful, but the invocation
  /// is old & stale.
  #[default]
  Stale,
  /// Emitted when the retrieval failed altogether.
  Failure(anyhow::Error),
}

#[allow(unused)]
impl LoaderResult {
  pub fn is_success(&self) -> bool {
    matches!(self, Self::Success)
  }
}
