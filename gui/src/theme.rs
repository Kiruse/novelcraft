use gpui::{Global, Rgba};

#[derive(Debug, Clone)]
pub struct Theme {
  pub bg: Rgba,
  pub text: Rgba,
}

impl Theme {
  pub fn dark() -> Self {
    Theme {
      bg: Rgba::try_from("#283333").unwrap(),
      text: Rgba::try_from("#E1F5F5").unwrap(),
    }
  }
}

impl Global for Theme {}

impl Default for Theme {
  fn default() -> Self {
    Self::dark()
  }
}
