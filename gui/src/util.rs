use std::fmt::Display;

pub use log::Level as LogLevel;

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
