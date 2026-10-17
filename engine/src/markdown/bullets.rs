//! Markdown bullet list utilities

use std::fmt::Display;

use crate::error::EngineError;

/// A markdown bullet list: one `- item` per line. Empty lines are ignored.
/// Unlike [super::TodoList], items carry no check-off semantics — chapter
/// notes are immutable records of events.
#[derive(Debug, Clone, Default, Eq, PartialEq)]
pub struct BulletList(pub Vec<String>);

impl BulletList {
  /// Parse a bullet list. Every non-empty line must be a `- `-prefixed item.
  pub fn parse(src: &str) -> Result<Self, EngineError> {
    let items = src
      .lines()
      .map(str::trim)
      .filter(|line| !line.is_empty())
      .map(|line| {
        line
          .strip_prefix("- ")
          .map(str::trim)
          .filter(|item| !item.is_empty())
          .map(str::to_string)
          .ok_or_else(|| EngineError::validation("expected bullet list item starting with \"- \""))
      })
      .collect::<Result<Vec<_>, _>>()?;
    Ok(Self(items))
  }

  #[inline(always)]
  pub fn items(&self) -> &[String] {
    &self.0
  }

  #[inline(always)]
  pub fn is_empty(&self) -> bool {
    self.0.is_empty()
  }

  #[inline(always)]
  pub fn len(&self) -> usize {
    self.0.len()
  }
}

impl Display for BulletList {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    for item in &self.0 {
      f.write_str("- ")?;
      f.write_str(item)?;
      f.write_str("\n")?;
    }
    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn parse_items() {
    let list = BulletList::parse("- one\n- two\n").unwrap();
    assert_eq!(list.items(), &["one".to_string(), "two".to_string()]);
    assert_eq!(list.to_string(), "- one\n- two\n");
  }

  #[test]
  fn parse_ignores_empty_lines_and_surrounding_whitespace() {
    let list = BulletList::parse("  - a \n\n   \n- b").unwrap();
    assert_eq!(list.items(), &["a".to_string(), "b".to_string()]);
  }

  #[test]
  fn parse_empty() {
    let list = BulletList::parse("\n \n").unwrap();
    assert!(list.is_empty());
    assert_eq!(list.to_string(), "");
  }

  #[test]
  fn parse_rejects_plain_lines() {
    assert!(BulletList::parse("- a\nplain line").is_err());
  }

  #[test]
  fn parse_rejects_bare_dash() {
    assert!(BulletList::parse("-").is_err());
    assert!(BulletList::parse("- ").is_err());
  }
}
