use std::fmt::Display;

pub use log::Level as LogLevel;

#[allow(unused)]
pub trait Loggable {
  fn log(&self, level: LogLevel);

  fn trace(&self)
  where
    Self: Sized,
  {
    self.log(LogLevel::Trace)
  }

  fn debug(&self)
  where
    Self: Sized,
  {
    self.log(LogLevel::Debug)
  }

  fn info(&self)
  where
    Self: Sized,
  {
    self.log(LogLevel::Info)
  }

  fn warn(&self)
  where
    Self: Sized,
  {
    self.log(LogLevel::Warn)
  }

  fn error(&self)
  where
    Self: Sized,
  {
    self.log(LogLevel::Error)
  }
}

impl<E: Display> Loggable for Result<(), E> {
  fn log(&self, level: LogLevel) {
    match self {
      Ok(_) => {},
      Err(e) => log::log!(level.into(), "Err: {e}"),
    }
  }
}
