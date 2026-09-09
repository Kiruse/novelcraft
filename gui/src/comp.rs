use gpui::{Action, Div, ElementId, Entity, Hsla, Rgba, Stateful, Text, div, px, text};
use gpui::prelude::*;

use crate::actions::{Back, ShowSettings};
use crate::text_input::TextInput;
use crate::theme::Theme;
use crate::util::HslaExt;

/// Creates the root component of most screens.
pub(crate) fn root(theme: &Theme) -> Div {
  div()
    .flex()
    .flex_col()
    .items_center()
    .w_full()
    .h_full()
    .bg(theme.bg)
    .text_color(theme.text)
}

pub(crate) fn settings_gear() -> impl IntoElement {
  div()
    .id("settings-gear")
    .absolute()
    .text_xl()
    .right_4()
    .cursor_pointer()
    .hover(|style| style.opacity(0.7))
    .on_click(|_, window, cx| window.dispatch_action(ShowSettings.boxed_clone(), cx))
    .child(text!("\u{2699}"))
}

pub(crate) fn btn_icon_close() -> impl IntoElement {
  div()
    .id("close-icon-btn")
    .absolute()
    .text_xl()
    .right_4()
    .cursor_pointer()
    .hover(|style| style.opacity(0.7))
    .on_click(|_, window, cx| window.dispatch_action(Back.boxed_clone(), cx))
    .child(text!("\u{00d7}"))
}

pub(crate) fn screen_root() -> Div {
  div()
    .flex()
    .flex_col()
    .items_center()
    .w_full()
    .h_full()
}

pub(crate) fn top_bar() -> Div {
  div()
    .relative()
    .w_full()
    .flex()
    .flex_row()
    .justify_center()
}

pub(crate) fn content() -> Stateful<Div> {
  div()
    .id("content")
    .relative()
    .w_full()
    .flex()
    .flex_col()
    .items_center()
    .overflow_y_scroll()
    .gap_4()
    .p_4()
    .max_w(px(640.))
}

#[inline(always)]
pub(crate) fn title(content: Text) -> Div {
  div().text_3xl().child(content)
}

#[inline(always)]
pub(crate) fn subtitle(content: Text) -> Div {
  div().text_2xl().child(content)
}

pub(crate) fn field(theme: &Theme, label: &str, input: &Entity<TextInput>) -> Div {
  div()
    .flex()
    .flex_col()
    .gap_1()
    .w_full()
    .child(div().text_color(theme.label).child(text!(label)))
    .child(input.clone())
}

pub(crate) fn field_group(theme: &Theme, label: &str) -> Div {
  div()
    .flex()
    .flex_col()
    .gap_2()
    .w_full()
    .p_3()
    .border_1()
    .border_color(theme.border)
    .rounded_sm()
    .child(div().text_lg().child(text!(label)))
}

pub(crate) fn create_text_input<T>(
  cx: &mut Context<'_, T>,
  multiline: bool,
  placeholder: &str,
) -> Entity<TextInput> {
  cx.new(|cx| {
    let mut input = TextInput::create(multiline, cx);
    input.placeholder = placeholder.into();
    input
  })
}

pub(crate) struct Button {
  anim_id: ElementId,
  label: Text,
  disabled: bool,
  fg: Rgba,
  bg: Rgba,
  fg_hover: Rgba,
  bg_hover: Rgba,
  bg_active: Rgba,
  bg_muted: Rgba,
  border: Rgba,
  border_hover: Rgba,
}

impl Button {
  #[inline(always)]
  pub fn disable(self, disabled: bool) -> Self {
    Self {
      disabled,
      ..self
    }
  }
}

impl IntoElement for Button {
  type Element = Stateful<Div>;
  fn into_element(self) -> Self::Element {
    let mut elem = div()
      .id(self.anim_id)
      .w_full()
      .p_2()
      .flex()
      .justify_center()
      .rounded_sm()
      .bg(self.bg)
      .text_color(self.fg)
      .border_1()
      .border_color(self.border)
      .child(self.label);
    if !self.disabled {
      elem = elem
        .cursor_pointer()
        .hover(move |style| style
          .text_color(self.fg_hover)
          .bg(self.bg_hover)
          .border_color(self.border_hover))
        .active(move |style| style.bg(self.bg_active));
    } else {
      elem = elem
        .bg(self.bg_muted)
        .text_color(Hsla::from(self.fg).opacity(0.9));
    }
    elem
  }
}

pub(crate) struct IncompleteButton {
  anim_id: ElementId,
  label: Text,
}

// Variants are only wired up as screens adopt them.
#[allow(dead_code)]
impl IncompleteButton {
  /// Inverted button: theme text color as background, theme bg as text.
  pub fn primary(&self, theme: &Theme) -> Button {
    Button {
      anim_id: self.anim_id.clone(),
      label: self.label.clone(),
      disabled: false,
      fg: theme.bg,
      bg: theme.text,
      fg_hover: theme.bg,
      bg_hover: theme.text.alpha(0.85),
      bg_active: Hsla::from(theme.text).lum(|l| l - 0.1).to_rgb(),
      bg_muted: theme.text.alpha(0.1),
      border: transparent(),
      border_hover: transparent(),
    }
  }

  /// Outlined button: theme text color for text & border, transparent bg.
  pub fn secondary(&self, theme: &Theme) -> Button {
    Button {
      anim_id: self.anim_id.clone(),
      label: self.label.clone(),
      disabled: false,
      fg: theme.text,
      bg: transparent(),
      fg_hover: theme.text,
      bg_hover: theme.text.alpha(0.1),
      bg_active: theme.text.alpha(0.05),
      bg_muted: theme.text.alpha(0.1),
      border: theme.text,
      border_hover: theme.text,
    }
  }

  /// Destructive primary: danger_bg as background, theme bg as text.
  pub fn danger_primary(&self, theme: &Theme) -> Button {
    Button {
      anim_id: self.anim_id.clone(),
      label: self.label.clone(),
      disabled: false,
      fg: theme.bg,
      bg: theme.danger_bg,
      fg_hover: theme.bg,
      bg_hover: Hsla::from(theme.danger_bg).lum(|l| l + 0.15).to_rgb(),
      bg_active: Hsla::from(theme.danger_bg).lum(|l| l - 0.1).to_rgb(),
      bg_muted: theme.danger_bg.alpha(0.1),
      border: transparent(),
      border_hover: transparent(),
    }
  }

  /// Destructive outlined: danger_fg for text & border, transparent bg.
  pub fn danger_secondary(&self, theme: &Theme) -> Button {
    let danger_hover = Hsla::from(theme.danger_fg)
      .lum(|l| l + 0.15)
      .to_rgb();
    Button {
      anim_id: self.anim_id.clone(),
      label: self.label.clone(),
      disabled: false,
      fg: theme.danger_fg,
      bg: transparent(),
      fg_hover: danger_hover.clone(),
      bg_hover: theme.danger_fg.alpha(0.1),
      bg_active: theme.danger_fg.alpha(0.2),
      bg_muted: theme.text.alpha(0.1),
      border: theme.danger_fg,
      border_hover: danger_hover,
    }
  }
}

pub(crate) fn button(
  anim_id: impl Into<ElementId>,
  label: Text,
) -> IncompleteButton {
  IncompleteButton { anim_id: anim_id.into(), label }
}

/// Fully transparent color — used to opt out of the border / background.
#[inline(always)]
pub(crate) fn transparent() -> Rgba {
  Rgba { r: 0., g: 0., b: 0., a: 0. }.alpha(0.)
}
