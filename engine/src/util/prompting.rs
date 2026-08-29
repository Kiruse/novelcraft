use std::{collections::HashMap, hash::Hash};

const INDENT_STEP: usize = 2;

pub type PromptifyResult = Result<(), PromptifyError>;

pub trait Promptify {
  fn promptify(&self, f: &mut PromptFormatter) -> PromptifyResult;

  fn to_prompt(&self) -> Result<String, PromptifyError> {
    let mut formatter = PromptFormatter::new();
    self.promptify(&mut formatter)?;
    Ok(formatter.finish())
  }
}

pub struct PromptFormatter {
  pub indent: usize,
  buf: String,
  at_line_start: bool,
}

impl Default for PromptFormatter {
  fn default() -> Self {
    Self {
      indent: 0,
      buf: String::new(),
      at_line_start: true,
    }
  }
}

impl PromptFormatter {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn new_indented(indent: usize) -> Self {
    Self {
      indent,
      buf: String::new(),
      at_line_start: true,
    }
  }

  pub fn indented<F: FnMut(&mut Self) -> PromptifyResult>(&mut self, mut cb: F) -> PromptifyResult {
    if !self.at_line_start {
      self.buf.push('\n');
      self.at_line_start = true;
    }
    self.indent += INDENT_STEP;
    let result = cb(self);
    self.indent -= INDENT_STEP;
    if !self.at_line_start {
      self.buf.push('\n');
      self.at_line_start = true;
    }
    result
  }

  pub fn write(&mut self, s: &str) -> PromptifyResult {
    let indent_str = " ".repeat(self.indent);
    let mut chars = s.chars().peekable();

    while self.at_line_start && matches!(chars.peek(), Some(&' ') | Some(&'\t')) {
      chars.next();
    }

    while let Some(ch) = chars.next() {
      if self.at_line_start && ch != '\n' {
        self.buf.push_str(&indent_str);
        self.at_line_start = false;
      }
      if ch == '\n' {
        self.buf.push('\n');
        self.at_line_start = true;
        while matches!(chars.peek(), Some(&' ') | Some(&'\t')) {
          chars.next();
        }
      } else {
        self.buf.push(ch);
      }
    }

    Ok(())
  }

  /// Write `s` to the formatter, reindenting to the formatter's current indentation,
  /// but preserving deeper indentations. Thus, if the base indentation is 4, followed
  /// by an indented block of 6, reindenting to 0 will set base indentation to 0 and
  /// block indentation to 2.
  pub fn write_reindent(&mut self, s: &str) -> PromptifyResult {
    let num_lines = s.lines().count();
    let min_indent = s.lines()
      .filter(|l| !l.trim().is_empty())
      .map(|l| l.len() - l.trim_start().len())
      .min()
      .unwrap_or(0);

    for (i, line) in s.lines().enumerate() {
      if i == 0 && !self.at_line_start {
        self.buf.push_str(line);
        if i < num_lines - 1 {
          self.buf.push('\n');
          self.at_line_start = true;
        }
        continue;
      }
      let trimmed = line.trim_start();
      if i > 0 {
        self.buf.push('\n');
      }
      if !trimmed.is_empty() {
        let original_indent = line.len() - trimmed.len();
        let new_indent = self.indent + original_indent.saturating_sub(min_indent);
        self.buf.push_str(&" ".repeat(new_indent));
        self.buf.push_str(trimmed);
        self.at_line_start = false;
      } else {
        self.at_line_start = true;
      }
    }

    Ok(())
  }

  #[inline]
  pub fn writeline(&mut self, s: &str) -> PromptifyResult {
    self.write(s)?;
    self.newline()
  }

  #[inline(always)]
  pub fn newline(&mut self) -> PromptifyResult {
    self.write("\n")
  }

  #[inline(always)]
  pub fn as_str(&self) -> &str {
    &self.buf
  }

  #[inline(always)]
  pub fn finish(self) -> String {
    self.buf
  }
}

#[derive(Debug, thiserror::Error)]
pub enum PromptifyError {
  #[error("generic promptify error: {0}")]
  Generic(String),
}

impl PromptifyError {
  pub fn generic(msg: impl Into<String>) -> Self {
    Self::Generic(msg.into())
  }
}

impl<T: ?Sized + Promptify> Promptify for &T {
  fn promptify(&self, f: &mut PromptFormatter) -> PromptifyResult {
    (**self).promptify(f)
  }
}

impl Promptify for str {
  fn promptify(&self, f: &mut PromptFormatter) -> PromptifyResult {
    f.write(self)
  }
}

impl Promptify for String {
  fn promptify(&self, f: &mut PromptFormatter) -> PromptifyResult {
    f.write(self)
  }
}

impl<K: Promptify + Hash, V: Promptify> Promptify for HashMap<K, V> {
  fn promptify(&self, f: &mut PromptFormatter) -> PromptifyResult {
    for (key, value) in self {
      key.promptify(f)?;
      f.write(": ")?;
      value.promptify(f)?;
      f.newline()?;
    }
    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::collections::HashMap;

  fn fmt() -> PromptFormatter {
    PromptFormatter::new()
  }

  fn fmt_indent(indent: usize) -> PromptFormatter {
    PromptFormatter::new_indented(indent)
  }

  #[test]
  fn write_simple() {
    let mut f = fmt();
    f.write("hello").unwrap();
    assert_eq!(f.as_str(), "hello");
  }

  #[test]
  fn write_empty() {
    let mut f = fmt();
    f.write("").unwrap();
    assert_eq!(f.as_str(), "");
  }

  #[test]
  fn write_preserves_trailing_whitespace_on_line() {
    let mut f = fmt();
    f.write("hello   ").unwrap();
    assert_eq!(f.as_str(), "hello   ");
  }

  #[test]
  fn write_newline_sets_line_start() {
    let mut f = fmt();
    f.write("a\n").unwrap();
    assert_eq!(f.as_str(), "a\n");
    assert!(f.at_line_start);
  }

  #[test]
  fn write_trims_whitespace_after_newline() {
    let mut f = fmt();
    f.write("a\n  b").unwrap();
    assert_eq!(f.as_str(), "a\nb");
  }

  #[test]
  fn write_indents_after_newline() {
    let mut f = fmt_indent(2);
    f.writeline("a").unwrap();
    f.writeline("b").unwrap();
    assert_eq!(f.as_str(), "  a\n  b\n");
  }

  #[test]
  fn write_indents_at_line_start() {
    let mut f = fmt_indent(2);
    f.write("hello").unwrap();
    assert_eq!(f.as_str(), "  hello");
    assert!(!f.at_line_start);
  }

  #[test]
  fn write_trims_leading_whitespace_at_line_start() {
    let mut f = fmt_indent(2);
    f.write("  hello").unwrap();
    assert_eq!(f.as_str(), "  hello");
  }

  #[test]
  fn write_only_whitespace_at_line_start() {
    let mut f = fmt();
    f.write("   \t  ").unwrap();
    assert_eq!(f.as_str(), "");
    assert!(f.at_line_start);
  }

  #[test]
  fn write_multiple_lines_with_indent() {
    let mut f = fmt_indent(2);
    f.write("line1\n  line2\n\tline3").unwrap();
    assert_eq!(f.as_str(), "  line1\n  line2\n  line3");
  }

  #[test]
  fn write_consecutive_newlines() {
    let mut f = fmt_indent(2);
    f.write("a\n\nb").unwrap();
    assert_eq!(f.as_str(), "  a\n\n  b");
  }

  #[test]
  fn write_consecutive_newlines_with_whitespace() {
    let mut f = fmt_indent(2);
    f.write("a\n  \n  b").unwrap();
    assert_eq!(f.as_str(), "  a\n\n  b");
  }

  #[test]
  fn write_reindent() {
    let mut f = fmt_indent(4);
    f.write("foo").unwrap();
    f.indented(|f| f.write("bar")).unwrap();
    f.write("baz").unwrap();
    assert_eq!(f.as_str(), "    foo\n      bar\n    baz");
    let s = f.finish();

    let mut f = fmt();
    f.write_reindent(&s).unwrap();
    assert_eq!(f.as_str(), "foo\n  bar\nbaz");
  }

  #[test]
  fn writeline() {
    let mut f = fmt();
    f.writeline("hello").unwrap();
    assert_eq!(f.as_str(), "hello\n");
  }

  #[test]
  fn newline() {
    let mut f = fmt();
    f.write("a").unwrap();
    f.newline().unwrap();
    assert_eq!(f.as_str(), "a\n");
    assert!(f.at_line_start);
  }

  #[test]
  fn indented_basic() {
    let mut f = fmt();
    f.write("root 1").unwrap();
    f.indented(|f| {
      f.write("child").unwrap();
      Ok(())
    }).unwrap();
    f.write("root 2").unwrap();
    assert_eq!(f.as_str(), "root 1\n  child\nroot 2");
  }

  #[test]
  fn indented_no_extra_newlines() {
    let mut f = fmt();
    f.writeline("root 1").unwrap();
    f.indented(|f| {
      f.writeline("child").unwrap();
      Ok(())
    }).unwrap();
    f.writeline("root 2").unwrap();
    assert_eq!(f.as_str(), "root 1\n  child\nroot 2\n");
  }

  #[test]
  fn indented_nested() {
    let mut f = fmt();
    f.write("L0\n").unwrap();
    f.indented(|f| {
      f.write("L1\n").unwrap();
      f.indented(|f| {
        f.write("L2\n").unwrap();
        Ok(())
      }).unwrap();
      f.write("L1b\n").unwrap();
      Ok(())
    }).unwrap();
    f.write("L0b").unwrap();
    assert_eq!(f.as_str(), "L0\n  L1\n    L2\n  L1b\nL0b");
  }

  #[test]
  fn indented_double() {
    let mut f = fmt();
    f.writeline("foo").unwrap();
    f.indented(|f| f.writeline("bar")).unwrap();
    f.indented(|f| f.writeline("baz")).unwrap();
    f.writeline("quux").unwrap();
    assert_eq!(f.as_str(), "foo\n  bar\n  baz\nquux\n");
  }

  #[test]
  fn indented_propagates_error() {
    let mut f = fmt();
    let result = f.indented(|_| Err(PromptifyError::generic("fail")));
    assert!(result.is_err());
  }

  #[test]
  fn indented_restores_indent_on_error() {
    let mut f = fmt();
    let _ = f.indented(|_| Err(PromptifyError::generic("fail")));
    assert_eq!(f.indent, 0);
  }

  #[test]
  fn indented_manual_whitespace_is_trimmed() {
    let mut f = fmt();
    f.write("root\n").unwrap();
    f.indented(|f| {
      f.write("    manually indented\n").unwrap();
      Ok(())
    }).unwrap();
    assert_eq!(f.as_str(), "root\n  manually indented\n");
  }

  #[test]
  fn indented_deeply_nested() {
    let mut f = fmt();
    f.write("a\n").unwrap();
    f.indented(|f| {
      f.write("b\n").unwrap();
      f.indented(|f| {
        f.write("c\n").unwrap();
        f.indented(|f| {
          f.write("d").unwrap();
          Ok(())
        }).unwrap();
        Ok(())
      }).unwrap();
      Ok(())
    }).unwrap();
    assert_eq!(f.as_str(), "a\n  b\n    c\n      d\n");
  }

  #[test]
  fn indented_empty_callback() {
    let mut f = fmt();
    f.write("root").unwrap();
    f.indented(|_| Ok(())).unwrap();
    assert_eq!(f.as_str(), "root\n");
    assert!(f.at_line_start);
  }

  #[test]
  fn indented_with_capturing_closure() {
    let label = "captured";
    let mut f = fmt();
    f.write("root\n").unwrap();
    f.indented(|f| {
      f.write(label).unwrap();
      Ok(())
    }).unwrap();
    assert_eq!(f.as_str(), "root\n  captured\n");
  }

  #[test]
  fn promptify_str() {
    assert_eq!("hello".to_prompt().unwrap(), "hello");
  }

  #[test]
  fn promptify_string() {
    assert_eq!("hello".to_string().to_prompt().unwrap(), "hello");
  }

  #[test]
  fn promptify_hashmap() {
    let mut m = HashMap::new();
    m.insert("name", "Alice");
    m.insert("age", "30");
    let mut f = PromptFormatter::new();
    f.write("Character:").unwrap();
    f.indented(|f| {
      m.promptify(f).unwrap();
      Ok(())
    }).unwrap();

    let mut lines = f.as_str().lines();
    let first = lines.next().unwrap();
    assert_eq!(first, "Character:");

    let remain = lines.collect::<Vec<_>>();
    assert!(remain.iter().find(|line| **line == "  name: Alice").is_some());
    assert!(remain.iter().find(|line| **line == "  age: 30").is_some());
  }
}
