use gpui::{Action, Div, Entity, Stateful, Text, div, px, text};
use gpui::prelude::*;

use crate::actions::{Back, ShowSettings};
use crate::text_input::TextInput;
use crate::theme::Theme;

/// Creates the root component of most screens.
pub(crate) fn root(theme: &Theme) -> Div {
  div()
    .flex()
    .flex_col()
    .items_center()
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
