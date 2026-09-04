#![allow(dead_code)]

use std::{ops::Range, rc::Rc};

use gpui::{
  App, Bounds, ClipboardItem, Context, CursorStyle, Element, ElementId, ElementInputHandler,
  Entity, EntityInputHandler, FocusHandle, Focusable, Font, GlobalElementId, Hsla,
  InspectorElementId, InteractiveElement, KeyBinding, LayoutId, MouseButton, MouseDownEvent,
  MouseMoveEvent, MouseUpEvent, PaintQuad, ParentElement, Pixels, Point, Render, ShapedLine,
  SharedString, Style, Styled, TextAlign, TextRun, UTF16Selection, UnderlineStyle, Window,
  WrappedLine, actions, div, fill, point, prelude::*, px, relative, size,
};
use unicode_segmentation::UnicodeSegmentation;

use crate::theme::Theme;

actions!(
  text_input,
  [
    Backspace,
    Delete,
    Left,
    Right,
    Up,
    Down,
    SelectLeft,
    SelectRight,
    SelectUp,
    SelectDown,
    SelectAll,
    Home,
    End,
    ShowCharacterPalette,
    Paste,
    Cut,
    Copy,
    Enter,
    Submit,
  ]
);

pub(crate) fn init(cx: &mut App) {
  cx.bind_keys([
    KeyBinding::new("backspace", Backspace, Some("TextInput")),
    KeyBinding::new("delete", Delete, Some("TextInput")),
    KeyBinding::new("left", Left, Some("TextInput")),
    KeyBinding::new("right", Right, Some("TextInput")),
    KeyBinding::new("up", Up, Some("TextInput")),
    KeyBinding::new("down", Down, Some("TextInput")),
    KeyBinding::new("shift-left", SelectLeft, Some("TextInput")),
    KeyBinding::new("shift-right", SelectRight, Some("TextInput")),
    KeyBinding::new("shift-up", SelectUp, Some("TextInput")),
    KeyBinding::new("shift-down", SelectDown, Some("TextInput")),
    KeyBinding::new("ctrl-a", SelectAll, Some("TextInput")),
    KeyBinding::new("cmd-a", SelectAll, Some("TextInput")),
    KeyBinding::new("home", Home, Some("TextInput")),
    KeyBinding::new("end", End, Some("TextInput")),
    KeyBinding::new("ctrl-v", Paste, Some("TextInput")),
    KeyBinding::new("cmd-v", Paste, Some("TextInput")),
    KeyBinding::new("ctrl-c", Copy, Some("TextInput")),
    KeyBinding::new("cmd-c", Copy, Some("TextInput")),
    KeyBinding::new("ctrl-x", Cut, Some("TextInput")),
    KeyBinding::new("cmd-x", Cut, Some("TextInput")),
    KeyBinding::new("enter", Enter, Some("TextInput")),
    KeyBinding::new("ctrl-enter", Submit, Some("TextInput")),
    KeyBinding::new("ctrl-cmd-space", ShowCharacterPalette, Some("TextInput")),
  ]);
}

pub(crate) type SubmitCallback = Rc<dyn Fn(&str, &mut Window, &mut App)>;

pub(crate) struct TextInput {
  pub multiline: bool,
  pub placeholder: SharedString,
  pub on_submit: Option<SubmitCallback>,
  focus_handle: FocusHandle,
  content: SharedString,
  selected_range: Range<usize>,
  selection_reversed: bool,
  marked_range: Option<Range<usize>>,
  last_layout: Option<ShapedLine>,
  last_multiline: Vec<MultilineLine>,
  last_bounds: Option<Bounds<Pixels>>,
  last_wrap_width: Option<Pixels>,
  is_selecting: bool,
}

struct MultilineLine {
  start: usize,
  line: WrappedLine,
}

impl TextInput {
  pub(crate) fn create(multiline: bool, cx: &mut Context<Self>) -> Self {
    Self {
      multiline,
      placeholder: "".into(),
      on_submit: None,
      focus_handle: cx.focus_handle(),
      content: "".into(),
      selected_range: 0..0,
      selection_reversed: false,
      marked_range: None,
      last_layout: None,
      last_multiline: Vec::new(),
      last_bounds: None,
      last_wrap_width: None,
      is_selecting: false,
    }
  }

  pub(crate) fn value(&self) -> &str {
    &self.content
  }

  pub(crate) fn set_value(&mut self, value: impl Into<SharedString>, cx: &mut Context<Self>) {
    self.content = value.into();
    self.selected_range = 0..0;
    self.selection_reversed = false;
    self.marked_range = None;
    cx.notify();
  }

  pub(crate) fn reset(&mut self, cx: &mut Context<Self>) {
    self.content = "".into();
    self.selected_range = 0..0;
    self.selection_reversed = false;
    self.marked_range = None;
    self.last_layout = None;
    self.last_multiline.clear();
    self.last_bounds = None;
    self.last_wrap_width = None;
    self.is_selecting = false;
    cx.notify();
  }

  fn submit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    if let Some(on_submit) = self.on_submit.clone() {
      on_submit(&self.content, window, cx);
    }
    cx.notify();
  }

  fn left(&mut self, _: &Left, _: &mut Window, cx: &mut Context<Self>) {
    if self.selected_range.is_empty() {
      self.move_to(self.previous_boundary(self.cursor_offset()), cx);
    } else {
      self.move_to(self.selected_range.start, cx)
    }
  }

  fn right(&mut self, _: &Right, _: &mut Window, cx: &mut Context<Self>) {
    if self.selected_range.is_empty() {
      self.move_to(self.next_boundary(self.selected_range.end), cx);
    } else {
      self.move_to(self.selected_range.end, cx)
    }
  }

  fn up(&mut self, _: &Up, window: &mut Window, cx: &mut Context<Self>) {
    self.move_vertically(-1, false, window, cx);
  }

  fn down(&mut self, _: &Down, window: &mut Window, cx: &mut Context<Self>) {
    self.move_vertically(1, false, window, cx);
  }

  fn select_up(&mut self, _: &SelectUp, window: &mut Window, cx: &mut Context<Self>) {
    self.move_vertically(-1, true, window, cx);
  }

  fn select_down(&mut self, _: &SelectDown, window: &mut Window, cx: &mut Context<Self>) {
    self.move_vertically(1, true, window, cx);
  }

  fn move_vertically(
    &mut self,
    delta: isize,
    select: bool,
    window: &mut Window,
    cx: &mut Context<Self>,
  ) {
    if !self.multiline || self.last_multiline.is_empty() {
      return;
    }
    let line_height = window.line_height();
    let (x, row) = self.cursor_visual_position(line_height);
    let total = self.total_rows() as isize;
    let target = row as isize + delta;
    let offset = if target < 0 {
      0
    } else if target >= total {
      self.content.len()
    } else {
      self
        .closest_index_at_row(x, target as usize, line_height)
        .unwrap_or(self.cursor_offset())
    };
    if select {
      self.select_to(offset, cx);
    } else {
      self.move_to(offset, cx);
    }
  }

  fn cursor_visual_position(&self, line_height: Pixels) -> (Pixels, usize) {
    let offset = self.cursor_offset();
    let Some((entry_ix, local)) = line_entry_index_for_offset(&self.last_multiline, offset) else {
      return (px(0.), 0);
    };
    let entry = &self.last_multiline[entry_ix];
    let row = row_for_local(&entry.line, local);
    let x = x_for_local_in_row(&entry.line, local, row, line_height);
    (x, rows_before(&self.last_multiline, entry_ix) + row)
  }

  fn total_rows(&self) -> usize {
    self
      .last_multiline
      .iter()
      .map(|entry| entry.line.wrap_boundaries().len() + 1)
      .sum::<usize>()
      .max(1)
  }

  fn closest_index_at_row(&self, x: Pixels, row: usize, line_height: Pixels) -> Option<usize> {
    let mut row_acc = 0;
    for entry in &self.last_multiline {
      let rows = entry.line.wrap_boundaries().len() + 1;
      if row < row_acc + rows {
        let local_y = line_height * ((row - row_acc) as f32 + 0.5);
        let local = entry
          .line
          .closest_index_for_position(point(x, local_y), line_height)
          .unwrap_or_else(|closest| closest);
        return Some((entry.start + local).min(self.content.len()));
      }
      row_acc += rows;
    }
    None
  }

  fn select_left(&mut self, _: &SelectLeft, _: &mut Window, cx: &mut Context<Self>) {
    self.select_to(self.previous_boundary(self.cursor_offset()), cx);
  }

  fn select_right(&mut self, _: &SelectRight, _: &mut Window, cx: &mut Context<Self>) {
    self.select_to(self.next_boundary(self.selected_range.end), cx);
  }

  fn select_all(&mut self, _: &SelectAll, _: &mut Window, cx: &mut Context<Self>) {
    self.move_to(0, cx);
    self.select_to(self.content.len(), cx)
  }

  fn home(&mut self, _: &Home, _: &mut Window, cx: &mut Context<Self>) {
    let target = if self.multiline {
      line_entry_index_for_offset(&self.last_multiline, self.cursor_offset())
        .map(|(ix, _)| self.last_multiline[ix].start)
        .unwrap_or(0)
    } else {
      0
    };
    self.move_to(target, cx);
  }

  fn end(&mut self, _: &End, _: &mut Window, cx: &mut Context<Self>) {
    let target = if self.multiline {
      line_entry_index_for_offset(&self.last_multiline, self.cursor_offset())
        .map(|(ix, _)| self.last_multiline[ix].start + self.last_multiline[ix].line.len())
        .unwrap_or(self.content.len())
    } else {
      self.content.len()
    };
    self.move_to(target, cx);
  }

  fn enter(&mut self, _: &Enter, window: &mut Window, cx: &mut Context<Self>) {
    if self.multiline {
      self.replace_text_in_range(None, "\n", window, cx);
    } else {
      self.submit(window, cx);
    }
  }

  fn submit_action(&mut self, _: &Submit, window: &mut Window, cx: &mut Context<Self>) {
    self.submit(window, cx);
  }

  fn backspace(&mut self, _: &Backspace, window: &mut Window, cx: &mut Context<Self>) {
    if self.selected_range.is_empty() {
      let prev = self.previous_boundary(self.cursor_offset());
      if self.cursor_offset() == prev {
        window.play_system_bell();
        return;
      }
      self.select_to(prev, cx)
    }
    self.replace_text_in_range(None, "", window, cx)
  }

  fn delete(&mut self, _: &Delete, window: &mut Window, cx: &mut Context<Self>) {
    if self.selected_range.is_empty() {
      let next = self.next_boundary(self.cursor_offset());
      if self.cursor_offset() == next {
        window.play_system_bell();
        return;
      }
      self.select_to(next, cx)
    }
    self.replace_text_in_range(None, "", window, cx)
  }

  fn on_mouse_down(&mut self, event: &MouseDownEvent, window: &mut Window, cx: &mut Context<Self>) {
    self.is_selecting = true;
    let line_height = window.line_height();

    if event.modifiers.shift {
      self.select_to(
        self.index_for_mouse_position(event.position, line_height),
        cx,
      );
    } else {
      self.move_to(
        self.index_for_mouse_position(event.position, line_height),
        cx,
      )
    }
  }

  fn on_mouse_up(&mut self, _: &MouseUpEvent, _window: &mut Window, _: &mut Context<Self>) {
    self.is_selecting = false;
  }

  fn on_mouse_move(&mut self, event: &MouseMoveEvent, window: &mut Window, cx: &mut Context<Self>) {
    if self.is_selecting {
      let line_height = window.line_height();
      self.select_to(
        self.index_for_mouse_position(event.position, line_height),
        cx,
      );
    }
  }

  fn show_character_palette(
    &mut self,
    _: &ShowCharacterPalette,
    window: &mut Window,
    _: &mut Context<Self>,
  ) {
    window.show_character_palette();
  }

  fn paste(&mut self, _: &Paste, window: &mut Window, cx: &mut Context<Self>) {
    if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
      self.replace_text_in_range(None, &text, window, cx);
    }
  }

  fn copy(&mut self, _: &Copy, _: &mut Window, cx: &mut Context<Self>) {
    if !self.selected_range.is_empty() {
      cx.write_to_clipboard(ClipboardItem::new_string(
        self.content[self.selected_range.clone()].to_string(),
      ));
    }
  }

  fn cut(&mut self, _: &Cut, window: &mut Window, cx: &mut Context<Self>) {
    if !self.selected_range.is_empty() {
      cx.write_to_clipboard(ClipboardItem::new_string(
        self.content[self.selected_range.clone()].to_string(),
      ));
      self.replace_text_in_range(None, "", window, cx)
    }
  }

  fn move_to(&mut self, offset: usize, cx: &mut Context<Self>) {
    self.selected_range = offset..offset;
    cx.notify()
  }

  fn cursor_offset(&self) -> usize {
    if self.selection_reversed {
      self.selected_range.start
    } else {
      self.selected_range.end
    }
  }

  fn index_for_mouse_position(&self, position: Point<Pixels>, line_height: Pixels) -> usize {
    if self.content.is_empty() {
      return 0;
    }

    let Some(bounds) = self.last_bounds.as_ref() else {
      return 0;
    };
    if position.y < bounds.top() {
      return 0;
    }
    if position.y > bounds.bottom() {
      return self.content.len();
    }

    if !self.multiline {
      let Some(line) = self.last_layout.as_ref() else {
        return 0;
      };
      return line.closest_index_for_x(position.x - bounds.left());
    }

    if self.last_multiline.is_empty() {
      return 0;
    }

    let x = position.x - bounds.left();
    let y = position.y - bounds.top();
    let mut row_offset = px(0.);
    for (ix, entry) in self.last_multiline.iter().enumerate() {
      let rows = entry.line.wrap_boundaries().len() + 1;
      let span = line_height * rows as f32;
      let is_last = ix == self.last_multiline.len() - 1;
      if y < row_offset + span || is_last {
        let local_y = (y - row_offset).max(px(0.));
        let local = entry
          .line
          .closest_index_for_position(point(x, local_y), line_height)
          .unwrap_or_else(|closest| closest);
        return (entry.start + local).min(self.content.len());
      }
      row_offset += span;
    }
    self.content.len()
  }

  fn select_to(&mut self, offset: usize, cx: &mut Context<Self>) {
    if self.selection_reversed {
      self.selected_range.start = offset
    } else {
      self.selected_range.end = offset
    };
    if self.selected_range.end < self.selected_range.start {
      self.selection_reversed = !self.selection_reversed;
      self.selected_range = self.selected_range.end..self.selected_range.start;
    }
    cx.notify()
  }

  fn offset_from_utf16(&self, offset: usize) -> usize {
    let mut utf8_offset = 0;
    let mut utf16_count = 0;

    for ch in self.content.chars() {
      if utf16_count >= offset {
        break;
      }
      utf16_count += ch.len_utf16();
      utf8_offset += ch.len_utf8();
    }

    utf8_offset
  }

  fn offset_to_utf16(&self, offset: usize) -> usize {
    let mut utf16_offset = 0;
    let mut utf8_count = 0;

    for ch in self.content.chars() {
      if utf8_count >= offset {
        break;
      }
      utf8_count += ch.len_utf8();
      utf16_offset += ch.len_utf16();
    }

    utf16_offset
  }

  fn range_to_utf16(&self, range: &Range<usize>) -> Range<usize> {
    self.offset_to_utf16(range.start)..self.offset_to_utf16(range.end)
  }

  fn range_from_utf16(&self, range_utf16: &Range<usize>) -> Range<usize> {
    self.offset_from_utf16(range_utf16.start)..self.offset_from_utf16(range_utf16.end)
  }

  fn previous_boundary(&self, offset: usize) -> usize {
    self
      .content
      .grapheme_indices(true)
      .rev()
      .find_map(|(idx, _)| (idx < offset).then_some(idx))
      .unwrap_or(0)
  }

  fn next_boundary(&self, offset: usize) -> usize {
    self
      .content
      .grapheme_indices(true)
      .find_map(|(idx, _)| (idx > offset).then_some(idx))
      .unwrap_or(self.content.len())
  }

  fn filtered_text(&self, text: &str) -> String {
    if self.multiline {
      text.to_owned()
    } else {
      text.replace(['\n', '\r'], " ")
    }
  }
}

impl EntityInputHandler for TextInput {
  fn text_for_range(
    &mut self,
    range_utf16: Range<usize>,
    actual_range: &mut Option<Range<usize>>,
    _window: &mut Window,
    _cx: &mut Context<Self>,
  ) -> Option<String> {
    let range = self.range_from_utf16(&range_utf16);
    actual_range.replace(self.range_to_utf16(&range));
    Some(self.content[range].to_string())
  }

  fn selected_text_range(
    &mut self,
    _ignore_disabled_input: bool,
    _window: &mut Window,
    _cx: &mut Context<Self>,
  ) -> Option<UTF16Selection> {
    Some(UTF16Selection {
      range: self.range_to_utf16(&self.selected_range),
      reversed: self.selection_reversed,
    })
  }

  fn marked_text_range(
    &self,
    _window: &mut Window,
    _cx: &mut Context<Self>,
  ) -> Option<Range<usize>> {
    self
      .marked_range
      .as_ref()
      .map(|range| self.range_to_utf16(range))
  }

  fn unmark_text(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {
    self.marked_range = None;
  }

  fn replace_text_in_range(
    &mut self,
    range_utf16: Option<Range<usize>>,
    new_text: &str,
    _window: &mut Window,
    cx: &mut Context<Self>,
  ) {
    let new_text = self.filtered_text(new_text);
    let range = range_utf16
      .as_ref()
      .map(|range_utf16| self.range_from_utf16(range_utf16))
      .or(self.marked_range.clone())
      .unwrap_or(self.selected_range.clone());

    self.content =
      (self.content[0..range.start].to_owned() + &new_text + &self.content[range.end..]).into();
    self.selected_range = range.start + new_text.len()..range.start + new_text.len();
    self.marked_range.take();
    cx.notify();
  }

  fn replace_and_mark_text_in_range(
    &mut self,
    range_utf16: Option<Range<usize>>,
    new_text: &str,
    new_selected_range_utf16: Option<Range<usize>>,
    _window: &mut Window,
    cx: &mut Context<Self>,
  ) {
    let new_text = self.filtered_text(new_text);
    let range = range_utf16
      .as_ref()
      .map(|range_utf16| self.range_from_utf16(range_utf16))
      .or(self.marked_range.clone())
      .unwrap_or(self.selected_range.clone());

    self.content =
      (self.content[0..range.start].to_owned() + &new_text + &self.content[range.end..]).into();
    if !new_text.is_empty() {
      self.marked_range = Some(range.start..range.start + new_text.len());
    } else {
      self.marked_range = None;
    }
    self.selected_range = new_selected_range_utf16
      .as_ref()
      .map(|range_utf16| self.range_from_utf16(range_utf16))
      .map(|new_range| new_range.start + range.start..new_range.end + range.end)
      .unwrap_or_else(|| range.start + new_text.len()..range.start + new_text.len());

    cx.notify();
  }

  fn bounds_for_range(
    &mut self,
    range_utf16: Range<usize>,
    bounds: Bounds<Pixels>,
    window: &mut Window,
    _cx: &mut Context<Self>,
  ) -> Option<Bounds<Pixels>> {
    let range = self.range_from_utf16(&range_utf16);
    let line_height = window.line_height();

    if !self.multiline {
      let last_layout = self.last_layout.as_ref()?;
      return Some(Bounds::from_corners(
        point(
          bounds.left() + last_layout.x_for_index(range.start),
          bounds.top(),
        ),
        point(
          bounds.left() + last_layout.x_for_index(range.end),
          bounds.bottom(),
        ),
      ));
    }

    let (entry_ix, local_start) = line_entry_index_for_offset(&self.last_multiline, range.start)?;
    let entry = &self.last_multiline[entry_ix];
    let row = row_for_local(&entry.line, local_start);
    let x0 = x_for_local_in_row(&entry.line, local_start, row, line_height);
    let local_end = (local_start + (range.end - range.start)).min(entry.line.len());
    let x1 = x_for_local_in_row(&entry.line, local_end, row, line_height);
    let y = bounds.top() + line_height * (rows_before(&self.last_multiline, entry_ix) + row) as f32;
    Some(Bounds::from_corners(
      point(bounds.left() + x0, y),
      point(bounds.left() + x1, y + line_height),
    ))
  }

  fn character_index_for_point(
    &mut self,
    position: Point<Pixels>,
    window: &mut Window,
    _cx: &mut Context<Self>,
  ) -> Option<usize> {
    self.last_bounds?.localize(&position)?;
    let line_height = window.line_height();
    Some(self.offset_to_utf16(self.index_for_mouse_position(position, line_height)))
  }
}

struct TextElement {
  input: Entity<TextInput>,
}

struct PrepaintState {
  line: Option<ShapedLine>,
  multiline: Vec<MultilineLine>,
  cursor: Option<PaintQuad>,
  selection: Vec<PaintQuad>,
}

impl TextElement {
  fn shape_multiline(
    &self,
    display_text: SharedString,
    runs: &[TextRun],
    wrap_width: Pixels,
    window: &mut Window,
    _cx: &mut App,
  ) -> Vec<MultilineLine> {
    let font_size = window.text_style().font_size.to_pixels(window.rem_size());
    let Ok(shaped) =
      window
        .text_system()
        .shape_text(display_text, font_size, runs, Some(wrap_width), None)
    else {
      return Vec::new();
    };
    let mut start = 0;
    shaped
      .into_iter()
      .map(|line| {
        let entry = MultilineLine { start, line };
        start += entry.line.len() + 1;
        entry
      })
      .collect()
  }
}

impl IntoElement for TextElement {
  type Element = Self;

  fn into_element(self) -> Self::Element {
    self
  }
}

impl Element for TextElement {
  type RequestLayoutState = ();
  type PrepaintState = PrepaintState;

  fn id(&self) -> Option<ElementId> {
    None
  }

  fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
    None
  }

  fn request_layout(
    &mut self,
    _id: Option<&GlobalElementId>,
    _inspector_id: Option<&InspectorElementId>,
    window: &mut Window,
    cx: &mut App,
  ) -> (LayoutId, Self::RequestLayoutState) {
    let line_height = window.line_height();
    let mut style = Style::default();
    style.size.width = relative(1.).into();

    let height = if self.input.read(cx).multiline {
      let rows = {
        let input = self.input.read(cx);
        let theme = cx.global::<Theme>();
        let style = window.text_style();
        let (display_text, text_color) = if input.content.is_empty() {
          (
            input.placeholder.clone(),
            Hsla::from(theme.text.opacity(0.35)),
          )
        } else {
          (input.content.clone(), style.color)
        };
        let runs = ime_runs(
          &display_text,
          &input.marked_range.clone(),
          style.font(),
          text_color,
        );
        let font_size = style.font_size.to_pixels(window.rem_size());
        let wrap_width = input
          .last_wrap_width
          .unwrap_or_else(|| window.viewport_size().width);
        window
          .text_system()
          .shape_text(display_text, font_size, &runs, Some(wrap_width), None)
          .map(|shaped| {
            shaped
              .iter()
              .map(|line| line.wrap_boundaries().len() + 1)
              .sum::<usize>()
          })
          .unwrap_or(1)
          .max(1)
      };
      line_height * rows as f32
    } else {
      line_height
    };
    style.size.height = height.into();

    (window.request_layout(style, [], cx), ())
  }

  fn prepaint(
    &mut self,
    _id: Option<&GlobalElementId>,
    _inspector_id: Option<&InspectorElementId>,
    bounds: Bounds<Pixels>,
    _request_layout: &mut Self::RequestLayoutState,
    window: &mut Window,
    cx: &mut App,
  ) -> Self::PrepaintState {
    let theme = cx.global::<Theme>();
    let cursor_color: Hsla = theme.text.into();
    let selection_color: Hsla = theme.text.opacity(0.25).into();

    let input = self.input.read(cx);
    let content = input.content.clone();
    let selected_range = input.selected_range.clone();
    let marked_range = input.marked_range.clone();
    let cursor = input.cursor_offset();
    let multiline = input.multiline;
    let style = window.text_style();
    let line_height = window.line_height();

    let (display_text, text_color) = if content.is_empty() {
      (
        input.placeholder.clone(),
        Hsla::from(theme.text.opacity(0.35)),
      )
    } else {
      (content.clone(), style.color)
    };
    let runs = ime_runs(&display_text, &marked_range, style.font(), text_color);
    let font_size = style.font_size.to_pixels(window.rem_size());

    if !multiline {
      let line = window
        .text_system()
        .shape_line(display_text, font_size, &runs, None);

      let cursor_pos = line.x_for_index(cursor);
      let (selection, cursor) = if selected_range.is_empty() {
        (
          None,
          Some(fill(
            Bounds::new(
              point(bounds.left() + cursor_pos, bounds.top()),
              size(px(2.), line_height),
            ),
            cursor_color,
          )),
        )
      } else {
        (
          Some(fill(
            Bounds::from_corners(
              point(
                bounds.left() + line.x_for_index(selected_range.start),
                bounds.top(),
              ),
              point(
                bounds.left() + line.x_for_index(selected_range.end),
                bounds.top() + line_height,
              ),
            ),
            selection_color,
          )),
          None,
        )
      };
      PrepaintState {
        line: Some(line),
        multiline: Vec::new(),
        cursor,
        selection: selection.into_iter().collect(),
      }
    } else {
      let lines = self.shape_multiline(display_text, &runs, bounds.size.width, window, cx);

      let mut cursor_quad = None;
      let mut selection_quads = Vec::new();

      if selected_range.is_empty() {
        if let Some((entry_ix, local)) = line_entry_index_for_offset(&lines, cursor) {
          let entry = &lines[entry_ix];
          let row = row_for_local(&entry.line, local);
          let x = x_for_local_in_row(&entry.line, local, row, line_height);
          let y = bounds.top() + line_height * (rows_before(&lines, entry_ix) + row) as f32;
          cursor_quad = Some(fill(
            Bounds::new(point(bounds.left() + x, y), size(px(2.), line_height)),
            cursor_color,
          ));
        }
      } else {
        for (entry_ix, entry) in lines.iter().enumerate() {
          let line_start = entry.start;
          let line_end = entry.start + entry.line.len();
          let s = selected_range.start.max(line_start);
          let e = selected_range.end.min(line_end);
          if s >= e {
            continue;
          }
          let row_offset = line_height * rows_before(&lines, entry_ix) as f32;
          for (row, (row_start, row_end)) in wrap_row_ranges(&entry.line).into_iter().enumerate() {
            let s_local = s.max(line_start + row_start) - line_start;
            let e_local = e.min(line_start + row_end) - line_start;
            if s_local >= e_local {
              continue;
            }
            let x0 = x_for_local_in_row(&entry.line, s_local, row, line_height);
            let x1 = if e_local == row_end && row_end == entry.line.len() {
              entry
                .line
                .position_for_index(e_local, line_height)
                .map(|p| p.x)
                .unwrap_or(px(0.))
            } else {
              x_for_local_in_row(&entry.line, e_local, row, line_height)
            };
            let y = bounds.top() + row_offset + line_height * row as f32;
            selection_quads.push(fill(
              Bounds::from_corners(
                point(bounds.left() + x0, y),
                point(bounds.left() + x1, y + line_height),
              ),
              selection_color,
            ));
          }
        }
      }

      PrepaintState {
        line: None,
        multiline: lines,
        cursor: cursor_quad,
        selection: selection_quads,
      }
    }
  }

  fn paint(
    &mut self,
    _id: Option<&GlobalElementId>,
    _inspector_id: Option<&InspectorElementId>,
    bounds: Bounds<Pixels>,
    _request_layout: &mut Self::RequestLayoutState,
    prepaint: &mut Self::PrepaintState,
    window: &mut Window,
    cx: &mut App,
  ) {
    let focus_handle = self.input.read(cx).focus_handle.clone();
    window.handle_input(
      &focus_handle,
      ElementInputHandler::new(bounds, self.input.clone()),
      cx,
    );

    for quad in prepaint.selection.drain(..) {
      window.paint_quad(quad);
    }

    let line_height = window.line_height();
    let last_layout = prepaint.line.take();
    if let Some(line) = last_layout.as_ref() {
      line
        .paint(
          bounds.origin,
          line_height,
          TextAlign::Left,
          None,
          window,
          cx,
        )
        .unwrap();
    }

    let multiline = std::mem::take(&mut prepaint.multiline);
    let mut row_offset = 0;
    for entry in &multiline {
      let y = bounds.origin.y + line_height * row_offset as f32;
      entry
        .line
        .paint(
          point(bounds.origin.x, y),
          line_height,
          TextAlign::Left,
          None,
          window,
          cx,
        )
        .unwrap();
      row_offset += entry.line.wrap_boundaries().len() + 1;
    }

    if focus_handle.is_focused(window)
      && let Some(cursor) = prepaint.cursor.take()
    {
      window.paint_quad(cursor);
    }

    self.input.update(cx, |input, _cx| {
      input.last_layout = last_layout;
      input.last_multiline = multiline;
      input.last_bounds = Some(bounds);
      input.last_wrap_width = Some(bounds.size.width);
    });
  }
}

fn ime_runs(
  display_text: &str,
  marked_range: &Option<Range<usize>>,
  font: Font,
  color: Hsla,
) -> Vec<TextRun> {
  let run = TextRun {
    len: display_text.len(),
    font,
    color,
    background_color: None,
    underline: None,
    strikethrough: None,
  };
  if let Some(marked_range) = marked_range {
    vec![
      TextRun {
        len: marked_range.start,
        ..run.clone()
      },
      TextRun {
        len: marked_range.end - marked_range.start,
        underline: Some(UnderlineStyle {
          color: Some(color),
          thickness: px(1.),
          wavy: false,
        }),
        ..run.clone()
      },
      TextRun {
        len: display_text.len() - marked_range.end,
        ..run
      },
    ]
    .into_iter()
    .filter(|run| run.len > 0)
    .collect()
  } else {
    vec![run]
  }
}

fn line_entry_index_for_offset(lines: &[MultilineLine], offset: usize) -> Option<(usize, usize)> {
  if lines.is_empty() {
    return None;
  }
  for (ix, entry) in lines.iter().enumerate() {
    let end = entry.start + entry.line.len();
    if offset <= end || ix == lines.len() - 1 {
      let local = offset.saturating_sub(entry.start).min(entry.line.len());
      return Some((ix, local));
    }
  }
  None
}

fn rows_before(lines: &[MultilineLine], entry_ix: usize) -> usize {
  lines[..entry_ix]
    .iter()
    .map(|entry| entry.line.wrap_boundaries().len() + 1)
    .sum()
}

fn wrap_row_ranges(line: &WrappedLine) -> Vec<(usize, usize)> {
  let mut ends: Vec<usize> = line
    .wrap_boundaries()
    .iter()
    .map(|boundary| line.runs()[boundary.run_ix].glyphs[boundary.glyph_ix].index)
    .collect();
  ends.push(line.len());
  let mut start = 0;
  ends
    .into_iter()
    .map(|end| {
      let range = (start, end);
      start = end;
      range
    })
    .collect()
}

fn row_for_local(line: &WrappedLine, local: usize) -> usize {
  let ranges = wrap_row_ranges(line);
  for (row, (_, end)) in ranges.iter().enumerate() {
    let is_last = row == ranges.len() - 1;
    if local < *end || (is_last && local <= *end) {
      return row;
    }
  }
  0
}

fn x_for_local_in_row(line: &WrappedLine, local: usize, row: usize, line_height: Pixels) -> Pixels {
  let row_start = wrap_row_ranges(line)[row].0;
  if local <= row_start {
    return px(0.);
  }
  line
    .position_for_index(local.min(line.len()), line_height)
    .map(|p| p.x)
    .unwrap_or(px(0.))
}

impl Render for TextInput {
  fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    let theme = cx.global::<Theme>();
    div()
      .key_context("TextInput")
      .track_focus(&self.focus_handle(cx))
      .cursor(CursorStyle::IBeam)
      .w_full()
      .on_action(cx.listener(Self::backspace))
      .on_action(cx.listener(Self::delete))
      .on_action(cx.listener(Self::left))
      .on_action(cx.listener(Self::right))
      .on_action(cx.listener(Self::up))
      .on_action(cx.listener(Self::down))
      .on_action(cx.listener(Self::select_left))
      .on_action(cx.listener(Self::select_right))
      .on_action(cx.listener(Self::select_up))
      .on_action(cx.listener(Self::select_down))
      .on_action(cx.listener(Self::select_all))
      .on_action(cx.listener(Self::home))
      .on_action(cx.listener(Self::end))
      .on_action(cx.listener(Self::show_character_palette))
      .on_action(cx.listener(Self::paste))
      .on_action(cx.listener(Self::cut))
      .on_action(cx.listener(Self::copy))
      .on_action(cx.listener(Self::enter))
      .on_action(cx.listener(Self::submit_action))
      .on_mouse_down(MouseButton::Left, cx.listener(Self::on_mouse_down))
      .on_mouse_up(MouseButton::Left, cx.listener(Self::on_mouse_up))
      .on_mouse_up_out(MouseButton::Left, cx.listener(Self::on_mouse_up))
      .on_mouse_move(cx.listener(Self::on_mouse_move))
      .child(
        div()
          .w_full()
          .p_2()
          .bg(theme.bg)
          .border_1()
          .border_color(theme.text.opacity(0.25))
          .text_color(theme.text)
          .child(TextElement { input: cx.entity() }),
      )
  }
}

impl Focusable for TextInput {
  fn focus_handle(&self, _: &App) -> FocusHandle {
    self.focus_handle.clone()
  }
}
