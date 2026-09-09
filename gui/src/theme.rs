use std::str::FromStr;

use gpui::{Global, Rgba};
use novelcraft_engine::error::EngineError;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Unique theme identifier
#[derive(Debug, Clone, Default)]
pub enum ThemeKind {
  #[default]
  Dark,
}

impl ThemeKind {
  pub fn as_str(&self) -> &str {
    match self {
      Self::Dark => "dark",
    }
  }
}

impl FromStr for ThemeKind {
  type Err = EngineError;
  fn from_str(s: &str) -> Result<Self, Self::Err> {
    match s {
      "dark" => Ok(Self::Dark),
      _ => Err(EngineError::Parse(format!("Unknown theme {s}"))),
    }
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Theme {
  #[serde(skip)]
  kind: ThemeKind,
  pub bg: Rgba,
  pub text: Rgba,
  /// Color of label text
  pub label: Rgba,
  pub border: Rgba,
  /// Background color of destructive UI elements
  pub danger_bg: Rgba,
  /// Foreground color of destructive UI elements (lighter than `danger_bg`)
  pub danger_fg: Rgba,
}

impl Theme {
  pub fn dark() -> Self {
    Theme {
      kind: ThemeKind::Dark,
      bg: Rgba::try_from("#283333").unwrap(),
      text: Rgba::try_from("#E1F5F5").unwrap(),
      label: Rgba::try_from("#E87813").unwrap(),
      border: Rgba::try_from("#666").unwrap().alpha(0.25),
      danger_bg: Rgba::try_from("#642C2C").unwrap(),
      danger_fg: Rgba::try_from("#E88C8C").unwrap(),
    }
  }

  pub fn by_name(name: &str) -> Result<Self, EngineError> {
    let kind: ThemeKind = name.parse()?;
    match kind {
      ThemeKind::Dark => Ok(Self::dark()),
    }
  }
}

impl Global for Theme {}

impl Default for Theme {
  fn default() -> Self {
    Self::dark()
  }
}

pub(crate) fn serialize_theme_name<S: Serializer>(
  theme: &Theme,
  serializer: S,
) -> Result<S::Ok, S::Error> {
  serializer.serialize_str(theme.kind.as_str())
}

pub(crate) fn deserialize_theme_name<'de, D: Deserializer<'de>>(
  deserializer: D,
) -> Result<Theme, D::Error> {
  let kind = String::deserialize(deserializer)?;
  Theme::by_name(&kind)
    .map_err(|e| serde::de::Error::custom(format!("Failed to deserialize theme kind: {e}")))
}
