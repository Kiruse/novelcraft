//! Markdown utilities

use std::fmt::Display;

use serde::{Deserialize, Serialize};

use crate::error::EngineError;

#[derive(Debug, Clone, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct TodoItem {
  pub checked: bool,
  pub content: String,
}

impl TodoItem {
  /// Parse a single markdown TODO list item.
  pub fn parse(src: &str) -> Result<Self, EngineError> {
    if src.contains("\n") {
      return Err(EngineError::input("expected single-line string"));
    }

    let checked = if src.starts_with("[ ]") {
      false
    } else if src.starts_with("[x]") {
      true
    } else {
      return Err(EngineError::validation("invalid todo item"));
    };

    Ok(TodoItem {
      checked,
      content: src[3..].trim().to_string(),
    })
  }

  pub fn to_string(&self) -> String { format!("{self}") }
}

impl Display for TodoItem {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    if self.checked {
      f.write_str("[x] ")?;
    } else {
      f.write_str("[ ] ")?;
    }
    f.write_str(&self.content)?;
    Ok(())
  }
}

#[derive(Debug, Clone)]
pub struct TodoList(pub Vec<TodoItem>);

impl TodoList {
  pub fn parse(src: &str) -> Result<Self, EngineError> {
    Ok(Self(src.lines().map(TodoItem::parse).collect::<Result<_, _>>()?))
  }

  pub fn to_string(&self) -> String { format!("{self}") }

  pub fn diff(&self, other: &Self) -> Vec<TodoListDiff> {
    let mut result = vec![];
    let mut self_differ = TodoListDiffer::new(&self);
    let mut other_differ = TodoListDiffer::new(&other);
    while !self_differ.is_empty() {
      // Items in both lists (checked, unchecked, unchanged)
      while self_differ.content_eq(&other_differ) {
        let self_item = self_differ.item().unwrap();
        let other_item = other_differ.item().unwrap();
        result.push(match (self_item.checked, other_item.checked) {
          (true, false) => TodoListDiff::Uncheck(self_item.clone()),
          (false, true) => TodoListDiff::Check(self_item.clone()),
          _ => TodoListDiff::Unchanged(self_item.clone()),
        });
        self_differ.next();
        other_differ.next();
      }

      // Interim items inserted
      if let Some(mut dist) = self_differ.find_offset(&other_differ) {
        while dist > 0 {
          result.push(TodoListDiff::Add(other_differ.item().unwrap().clone()));
          other_differ.next();
          dist -= 1;
        }
      }
      // Item removed
      else if let Some(item) = self_differ.item() {
        result.push(TodoListDiff::Remove(item.clone()));
        self_differ.next();
      }
    }

    // Self list exhausted, add remaining items
    while let Some(item) = other_differ.item() {
      result.push(TodoListDiff::Add(item.clone()));
      other_differ.next();
    }

    result
  }
}

impl Display for TodoList {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    for (i, item) in self.0.iter().enumerate() {
      item.fmt(f)?;
      if i < self.0.len() - 1 {
        f.write_str("\n")?;
      }
    }
    Ok(())
  }
}

#[derive(Debug, Clone)]
pub enum TodoListDiff {
  /// Given [TodoItem] has not been changed from reference list.
  Unchanged(TodoItem),
  /// Given [TodoItem] has been added to reference list.
  Add(TodoItem),
  /// Given [TodoItem] has been removed from reference list.
  Remove(TodoItem),
  /// Given [TodoItem] has been checked from reference list.
  Check(TodoItem),
  /// Given [TodoItem] has been unchecked from reference list.
  Uncheck(TodoItem),
}

struct TodoListDiffer<'a> {
  src: &'a TodoList,
  idx: usize,
}

impl<'a> TodoListDiffer<'a> {
  fn new(src: &'a TodoList) -> Self {
    Self { src, idx: 0 }
  }

  fn next(&mut self) -> Option<&'a TodoItem> {
    self.idx += 1;
    self.item()
  }

  /// Convenience function for getting the *total* length of the source vec.
  #[inline(always)]
  fn len(&self) -> usize {
    self.src.0.len()
  }

  /// Convenience function for getting the *remaining* length of the source
  /// vec from the current index.
  #[inline(always)]
  fn remain(&self) -> usize {
    self.len() - self.idx
  }

  /// Get the current item
  #[inline(always)]
  fn item(&self) -> Option<&'a TodoItem> {
    self.src.0.get(self.idx)
  }

  /// Get the item at index `self.idx + n`.
  #[inline(always)]
  fn nth(&self, n: usize) -> Option<&'a TodoItem> {
    self.src.0.get(self.idx + n)
  }

  /// Check if the number of remaining items is 0
  #[inline(always)]
  fn is_empty(&self) -> bool {
    self.remain() == 0
  }

  /// Seek for an occurrence of this current item in the other differ
  /// starting at the other differ's current index. Disregards checked
  /// state. Returns the offset from the current index at which the
  /// item was found in `other`.
  fn find_offset(&self, other: &TodoListDiffer) -> Option<usize> {
    let Some(self_item) = self.item() else { return None };

    for n in 0..other.remain() {
      let Some(other_item) = other.nth(n) else { return None };
      if self_item.content == other_item.content {
        return Some(n);
      }
    }
    None
  }

  /// Compare self item with other item for equality while ignoring checked state.
  fn content_eq(&self, other: &TodoListDiffer) -> bool {
    let Some(self_item) = self.item() else { return false };
    let Some(other_item) = other.item() else { return false };
    self_item.content == other_item.content
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::error::EngineError;

  fn item(checked: bool, content: &str) -> TodoItem {
    TodoItem { checked, content: content.to_string() }
  }

  fn list(items: Vec<TodoItem>) -> TodoList {
    TodoList(items)
  }

  // --- TodoItem::parse ---

  #[test]
  fn todo_item_parse_unchecked() {
    let parsed = TodoItem::parse("[ ] buy milk").unwrap();
    assert_eq!(parsed, item(false, "buy milk"));
  }

  #[test]
  fn todo_item_parse_checked() {
    let parsed = TodoItem::parse("[x] read book").unwrap();
    assert_eq!(parsed, item(true, "read book"));
  }

  #[test]
  fn todo_item_parse_trims_content() {
    let parsed = TodoItem::parse("[ ]   spaced out  ").unwrap();
    assert_eq!(parsed, item(false, "spaced out"));
  }

  #[test]
  fn todo_item_parse_rejects_multiline() {
    let err = TodoItem::parse("[ ] line1\n[ ] line2").unwrap_err();
    assert!(matches!(err, EngineError::Input(_)));
  }

  #[test]
  fn todo_item_parse_rejects_invalid_prefix() {
    let err = TodoItem::parse("[-] not a todo").unwrap_err();
    assert!(matches!(err, EngineError::Validation(_)));
  }

  // --- TodoList::parse ---

  #[test]
  fn todo_list_parse_multiple() {
    let parsed = TodoList::parse("[ ] first\n[x] second\n[ ] third").unwrap();
    assert_eq!(parsed.0.len(), 3);
    assert_eq!(parsed.0[0], item(false, "first"));
    assert_eq!(parsed.0[1], item(true, "second"));
    assert_eq!(parsed.0[2], item(false, "third"));
  }

  #[test]
  fn todo_list_parse_single() {
    let parsed = TodoList::parse("[x] only one").unwrap();
    assert_eq!(parsed.0.len(), 1);
    assert_eq!(parsed.0[0], item(true, "only one"));
  }

  #[test]
  fn todo_list_parse_empty_string() {
    let parsed = TodoList::parse("").unwrap();
    assert!(parsed.0.is_empty());
  }

  #[test]
  fn todo_list_parse_invalid_item_propagates_error() {
    let err = TodoList::parse("[ ] ok\n[-] bad").unwrap_err();
    assert!(matches!(err, EngineError::Validation(_)));
  }

  // --- TodoList::diff ---

  #[test]
  fn diff_unchanged() {
    let a = list(vec![item(false, "a"), item(true, "b"), item(false, "c")]);
    let b = a.clone();
    let diffs = a.diff(&b);
    assert_eq!(diffs.len(), 3);
    assert!(matches!(&diffs[0], TodoListDiff::Unchanged(i) if i.content == "a"));
    assert!(matches!(&diffs[1], TodoListDiff::Unchanged(i) if i.content == "b"));
    assert!(matches!(&diffs[2], TodoListDiff::Unchanged(i) if i.content == "c"));
  }

  #[test]
  fn diff_add_to_end() {
    let a = list(vec![item(false, "a"), item(false, "b")]);
    let b = list(vec![item(false, "a"), item(false, "b"), item(false, "c")]);
    let diffs = a.diff(&b);
    assert_eq!(diffs.len(), 3);
    assert!(matches!(&diffs[0], TodoListDiff::Unchanged(_)));
    assert!(matches!(&diffs[1], TodoListDiff::Unchanged(_)));
    assert!(matches!(&diffs[2], TodoListDiff::Add(i) if i.content == "c"));
  }

  #[test]
  fn diff_add_in_between() {
    let a = list(vec![item(false, "a"), item(false, "c")]);
    let b = list(vec![item(false, "a"), item(false, "b"), item(false, "c")]);
    let diffs = a.diff(&b);
    assert_eq!(diffs.len(), 3);
    assert!(matches!(&diffs[0], TodoListDiff::Unchanged(i) if i.content == "a"));
    assert!(matches!(&diffs[1], TodoListDiff::Add(i) if i.content == "b"));
    assert!(matches!(&diffs[2], TodoListDiff::Unchanged(i) if i.content == "c"));
  }

  #[test]
  fn diff_remove_only() {
    let a = list(vec![item(false, "a"), item(false, "b"), item(false, "c")]);
    let b = list(vec![item(false, "a"), item(false, "c")]);
    let diffs = a.diff(&b);
    assert_eq!(diffs.len(), 3);
    assert!(matches!(&diffs[0], TodoListDiff::Unchanged(i) if i.content == "a"));
    assert!(matches!(&diffs[1], TodoListDiff::Remove(i) if i.content == "b"));
    assert!(matches!(&diffs[2], TodoListDiff::Unchanged(i) if i.content == "c"));
  }

  #[test]
  fn diff_check_only() {
    let a = list(vec![item(false, "a"), item(false, "b")]);
    let b = list(vec![item(false, "a"), item(true, "b")]);
    let diffs = a.diff(&b);
    assert_eq!(diffs.len(), 2);
    assert!(matches!(&diffs[0], TodoListDiff::Unchanged(_)));
    assert!(matches!(&diffs[1], TodoListDiff::Check(i) if i.content == "b"));
  }

  #[test]
  fn diff_uncheck_only() {
    let a = list(vec![item(true, "a"), item(true, "b")]);
    let b = list(vec![item(true, "a"), item(false, "b")]);
    let diffs = a.diff(&b);
    assert_eq!(diffs.len(), 2);
    assert!(matches!(&diffs[0], TodoListDiff::Unchanged(_)));
    assert!(matches!(&diffs[1], TodoListDiff::Uncheck(i) if i.content == "b"));
  }

  #[test]
  fn diff_mixed() {
    let a = list(vec![
      item(false, "keep"),
      item(false, "remove me"),
      item(true,  "uncheck me"),
      item(false, "check me"),
      item(false, "stay"),
    ]);
    let b = list(vec![
      item(false, "keep"),
      item(false, "uncheck me"),
      item(false, "inserted"),
      item(true,  "check me"),
      item(false, "stay"),
    ]);
    let diffs = a.diff(&b);
    assert_eq!(diffs.len(), 6);
    assert!(matches!(&diffs[0], TodoListDiff::Unchanged(i) if i.content == "keep"));
    assert!(matches!(&diffs[1], TodoListDiff::Remove(i) if i.content == "remove me"));
    assert!(matches!(&diffs[2], TodoListDiff::Uncheck(i) if i.content == "uncheck me"));
    assert!(matches!(&diffs[3], TodoListDiff::Add(i) if i.content == "inserted"));
    assert!(matches!(&diffs[4], TodoListDiff::Check(i) if i.content == "check me"));
    assert!(matches!(&diffs[5], TodoListDiff::Unchanged(i) if i.content == "stay"));
  }
}
