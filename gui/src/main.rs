use gpui::*;
use gpui_platform::application;

struct AppRoot;

impl Render for AppRoot {
  fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
    div()
      .flex()
      .flex_col()
      .gap_3()
      .bg(rgb(0x1a1a2e))
      .size_full()
      .justify_center()
      .items_center()
      .child(
        div()
          .text_xl()
          .text_color(rgb(0xffffff))
          .child("NovelCraft"),
      )
      .child(
        div()
          .text_sm()
          .text_color(rgb(0x888888))
          .child("gpui + engine workspace"),
      )
  }
}

fn main() {
  application().run(|cx: &mut App| {
    cx.open_window(WindowOptions::default(), |_, cx| {
      cx.new(|_| AppRoot)
    })
    .unwrap();
    cx.activate(true);
  });
}
