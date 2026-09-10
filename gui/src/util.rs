use std::fmt::Display;

use gpui::{App, Hsla};
pub use log::Level as LogLevel;

use crate::{Toast, ToastVariant};

#[allow(unused)]
pub trait Loggable {
  fn log(&self, level: LogLevel);

  #[inline(always)]
  fn trace(&self) where Self: Sized {
    self.log(LogLevel::Trace)
  }

  #[inline(always)]
  fn debug(&self) where Self: Sized {
    self.log(LogLevel::Debug)
  }

  #[inline(always)]
  fn info(&self) where Self: Sized {
    self.log(LogLevel::Info)
  }

  #[inline(always)]
  fn warn(&self) where Self: Sized {
    self.log(LogLevel::Warn)
  }

  #[inline(always)]
  fn error(&self) where Self: Sized {
    self.log(LogLevel::Error)
  }
}

impl<T, E: Display> Loggable for Result<T, E> {
  fn log(&self, level: LogLevel) {
    match self {
      Ok(_) => {}
      Err(e) => log::log!(level.into(), "Err: {e}"),
    }
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
  fn report_toast(&self, variant: ToastVariant, cx: &mut App) -> &Self;
  /// Info toast (message from the value) on success, error toast on failure.
  fn info_toast(&self, cx: &mut App) -> &Self {
    self.report_toast(ToastVariant::Info, cx)
  }
  /// Success toast (message from the value) on success, error toast on failure.
  fn success_toast(&self, cx: &mut App) -> &Self {
    self.report_toast(ToastVariant::Success, cx)
  }
  /// Warning toast (message from the value) on success, error toast on failure.
  fn warn_toast(&self, cx: &mut App) -> &Self {
    self.report_toast(ToastVariant::Warn, cx)
  }
  /// Error toast on failure, nothing on success.
  fn error_toast(&self, cx: &mut App) -> &Self {
    self.report_toast(ToastVariant::Error, cx)
  }
}

impl<T, E: Display> Toastable for Result<T, E> {
  fn report_toast(&self, variant: ToastVariant, cx: &mut App) -> &Self {
    if let Err(e) = self {
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
pub enum Loadable<T> {
  Pending,
  Done(T),
}

impl<T> Loadable<T> {
  #[inline(always)]
  pub fn is_pending(&self) -> bool {
    matches!(self, Loadable::Pending)
  }

  #[inline(always)]
  pub fn is_done(&self) -> bool {
    matches!(self, Loadable::Done(_))
  }

  #[inline(always)]
  pub fn unwrap(self) -> T {
    match self {
      Loadable::Done(v) => v,
      Loadable::Pending => panic!("Pending loadable value"),
    }
  }
}
