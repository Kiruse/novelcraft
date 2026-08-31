use gpui::{
  Div, InteractiveElement, IntoElement, ParentElement, StatefulInteractiveElement, Styled, div, text,
};

use crate::{actions::ShowSettings, theme::Theme};

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
    .on_click(move |_, _, cx| cx.dispatch_action(&ShowSettings))
    .child(text!("\u{2699}"))
}
