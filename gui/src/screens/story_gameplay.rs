use std::rc::Rc;

use gpui::{
  AnyElement, App, Context, Div, DragMoveEvent, Entity, Hsla, IntoElement, MouseButton, Render, Stateful, Task, Window, div, prelude::*, px, text,
};
use novelcraft_engine::error::EngineError;
use novelcraft_engine::game::pages::PageV1;
use novelcraft_engine::game::session::SessionV1;
use novelcraft_engine::AgentMessageChunk;
use tokio::sync::{mpsc, oneshot};

use crate::comp::*;
use crate::text_input::TextInput;
use crate::theme::Theme;
use crate::util::{Loggable, Toastable};
use crate::{Command, EngineQuery, PageQuery, Profiles, StoryId, ToastVariant};

/// Swipe distance (in logical pixels) required to flip to the adjacent page.
const SWIPE_THRESHOLD: f32 = 60.;
/// Per-move swipe distance treated as a gesture boundary artifact (e.g. the
/// first move event after the pointer re-entered the window) rather than a
/// genuine drag movement.
const SWIPE_TELEPORT: f32 = SWIPE_THRESHOLD * 4.;
/// Width of the toggleable game state side bar.
const SIDEBAR_WIDTH: f32 = 280.;

/// Marker payload for drags started on the page viewport, used for
/// swipe-based page navigation.
#[derive(Debug, Copy, Clone)]
struct PageSwipe;

/// Invisible drag ghost — swipes are tracked via `DragMoveEvent`,
/// the ghost itself is never meant to be seen.
struct DragGhost;

impl Render for DragGhost {
  fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
    div()
  }
}

/// In-flight generation state, rendered in place of the page content.
#[derive(Debug, Default)]
struct Stream {
  prompt: String,
  text: String,
  activity: bool,
}

pub(crate) struct StoryGameplayScreen {
  pub id: StoryId,
  /// Keeps the query watcher alive — cancelled when the screen drops.
  #[allow(dead_code)]
  watch_task: Task<()>,
  stream: Option<Stream>,
  input: Entity<TextInput>,
  popover_open: bool,
  sidebar_open: bool,
  /// Accumulated horizontal distance of the active swipe gesture.
  swipe_x: f32,
  /// Horizontal pointer position of the previous swipe move event.
  swipe_last: Option<f32>,
}

impl StoryGameplayScreen {
  pub fn create(cx: &mut Context<Self>) -> Self {
    let mut rx_page = cx.global::<PageQuery>().rx();
    let mut rx_session = cx.global::<EngineQuery<SessionV1>>().rx();
    let mut rx_profile = cx.global::<EngineQuery<Profiles>>().rx();

    let watch_task = cx.spawn(async move |this, cx| {
      let mut running = true;
      while running {
        running = tokio::select! {
          Ok(()) = rx_page.changed() => true,
          Ok(()) = rx_session.changed() => true,
          Ok(()) = rx_profile.changed() => true,
          else => false, // only called when all receivers have been closed
        };
        cx.update(|cx| {
          cx.notify(this.entity_id());
        });
      }
    });

    let screen = Self {
      id: StoryId::default(),
      stream: None,
      watch_task,
      input: create_text_input(cx, true, "What do you do or say?"),
      popover_open: false,
      sidebar_open: false,
      swipe_x: 0.,
      swipe_last: None,
    };

    let this = cx.entity().downgrade();
    screen.input.update(cx, |input, _| {
      input.on_submit = Some(Rc::new(move |_, _, cx| {
        if let Some(this) = this.upgrade() {
          this.update(cx, |screen, cx| screen.submit_input(cx));
        }
      }));
    });

    screen
  }

  pub(crate) fn enter(&mut self, id: StoryId, cx: &mut Context<Self>) {
    self.id = id;
    self.popover_open = false;
    self.reset(cx);

    // Load the session on the engine thread, then refresh the cache. The
    // engine does not emit `SwitchSession` on failure, so the awaited
    // refresh observes both outcomes. Once published, open the last page.
    Command::PlaySession { id: self.id.clone() }.dispatch(cx);
    let mut rx_session = cx.global::<EngineQuery<SessionV1>>().refresh(cx);
    cx.global::<EngineQuery<Profiles>>().refresh(cx);

    cx.spawn(async move |this, cx| {
      let Ok(()) = rx_session.changed().await else { return };
      this.update(cx, |this, cx| {
        let Some(count) = this.session_page_count(cx) else { return };
        if count > 0 {
          this.goto(count - 1, cx);
        }
        cx.notify();
      }).ok();
    }).detach();
  }

  fn reset(&mut self, cx: &mut Context<Self>) {
    self.stream = None;
    self.swipe_x = 0.;
    self.swipe_last = None;
    // Discard the previous session's page & any in-flight load.
    cx.global_mut::<PageQuery>().clear();
    cx.notify();
  }

  /// The profile ID the gameplay session effectively runs with: the
  /// session's stored profile, falling back to the global one.
  fn active_profile_id(&self, cx: &App) -> Option<String> {
    let session = cx.global::<EngineQuery<SessionV1>>().curr();
    let profiles = cx.global::<EngineQuery<Profiles>>().curr();
    session
      .value()
      .and_then(|session| session.profile.clone())
      .or_else(|| {
        profiles
          .value()
          .and_then(|profiles| profiles.active_profile.clone())
      })
  }

  fn switch_profile(&mut self, id: &str, cx: &mut Context<Self>) {
    Command::SwitchProfile(id.to_string()).dispatch(cx);
    // The engine thread applies it to the config and the active session,
    // then persists — refresh both caches instead of mutating locally.
    cx.global::<EngineQuery<Profiles>>().refresh(cx);
    cx.global::<EngineQuery<SessionV1>>().refresh(cx);
    cx.notify();
  }

  fn session_page_count(&self, cx: &App) -> Option<usize> {
    cx
      .global::<EngineQuery<SessionV1>>()
      .curr()
      .value()
      .map(|session| session.page_count)
  }

  /// Load the page at `index` (clamped to the session's page count) into
  /// the global [PageQuery]. The watcher task re-renders once it publishes.
  fn goto(&mut self, index: usize, cx: &mut Context<Self>) {
    let Some(count) = self.session_page_count(cx) else {
      return;
    };
    if count == 0 {
      return;
    }

    cx.global::<PageQuery>().load(index.min(count - 1), cx);
    cx.notify();
  }

  pub(crate) fn navigate(&mut self, delta: isize, cx: &mut Context<Self>) {
    if self.stream.is_some() {
      return;
    }
    let Some(count) = self.session_page_count(cx) else {
      return;
    };
    if count == 0 {
      return;
    }
    let current = cx.global::<PageQuery>().page_index() as isize;
    let target = (current + delta).clamp(0, count as isize - 1) as usize;
    self.goto(target, cx);
  }

  fn jump_to_end(&mut self, cx: &mut Context<Self>) {
    let Some(count) = self.session_page_count(cx) else {
      return;
    };
    if count > 0 {
      self.goto(count - 1, cx);
    }
  }

  fn on_swipe(
    &mut self,
    ev: &DragMoveEvent<PageSwipe>,
    _window: &mut Window,
    cx: &mut Context<Self>,
  ) {
    let x = f32::from(ev.event.position.x);
    if let Some(last) = self.swipe_last {
      let delta = x - last;
      if delta.abs() > SWIPE_TELEPORT {
        self.swipe_last = Some(x);
        return;
      }
      self.swipe_x += delta;
    }
    self.swipe_last = Some(x);

    if self.swipe_x <= -SWIPE_THRESHOLD {
      self.swipe_x = 0.;
      self.swipe_last = None;
      self.navigate(1, cx);
    } else if self.swipe_x >= SWIPE_THRESHOLD {
      self.swipe_x = 0.;
      self.swipe_last = None;
      self.navigate(-1, cx);
    }
  }

  fn submit_input(&mut self, cx: &mut Context<Self>) {
    if self.stream.is_some() {
      return;
    }
    let content = self.input.read(cx).value().trim().to_string();
    if content.is_empty() {
      return;
    }
    self.input.update(cx, |input, cx| input.reset(cx));
    self.submit(content, cx);
  }

  fn submit(&mut self, content: String, cx: &mut Context<Self>) {
    let Some(count) = self.session_page_count(cx) else { return };

    let (chunks_tx, mut chunks_rx) = mpsc::channel::<AgentMessageChunk>(100);
    let (done_tx, done_rx) = oneshot::channel::<Result<(), EngineError>>();
    self.stream = Some(Stream {
      prompt: content.clone(),
      ..Default::default()
    });
    cx.notify();

    let page_idx = cx.global::<PageQuery>().page_index();

    Command::GamePrompt {
      content,
      // only supply when not on last page
      from_page: (page_idx + 1 < count).then_some(page_idx),
      chunks: chunks_tx,
      done: done_tx,
    }.dispatch(cx);

    let sid = self.id.0.clone();
    cx.spawn(async move |this, cx| {
      while let Some(chunk) = chunks_rx.recv().await {
        if this
          .update(cx, |screen, cx| {
            if screen.id.0 == sid && screen.stream.is_some() {
              screen.on_chunk(chunk, cx);
            }
          })
          .is_err()
        {
          return;
        }
      }

      if let Ok(res) = done_rx.await {
        this.update(cx, |this, cx| {
          if this.id.0 != sid { return };
          res.report_toast(ToastVariant::Error, cx).error().ok();
          this.stream = None;
          // A fork may have truncated pages, so refresh the session cache;
          // the current page gained the fresh response either way — reload
          // it directly, bypassing `goto`'s possibly-stale count guard.
          cx.global::<EngineQuery<SessionV1>>().refresh(cx);
          let current = cx.global::<PageQuery>().page_index();
          cx.global::<PageQuery>().load(current, cx);
          cx.notify();
        }).warn().ok();
      }
    }).detach();
  }

  fn on_chunk(&mut self, chunk: AgentMessageChunk, cx: &mut Context<Self>) {
    let Some(stream) = self.stream.as_mut() else { return };
    match chunk {
      AgentMessageChunk::Content { text } => stream.text.push_str(&text),
      AgentMessageChunk::ToolCallStart { .. } |
      AgentMessageChunk::ToolCallArgs { .. } => {
        stream.activity = true;
      }
      AgentMessageChunk::Reasoning { .. } |
      AgentMessageChunk::Done { .. } => {}
    }
    cx.notify();
  }

  fn top_bar_ui(&self, cx: &Context<'_, Self>) -> Div {
    let title_text = cx
      .global::<EngineQuery<SessionV1>>()
      .curr()
      .value()
      .map(|session| session.title.clone())
      .unwrap_or_else(|| "...".into());

    top_bar()
      .child(
        div()
          .id("sidebar-toggle")
          .absolute()
          .left_4()
          .text_xl()
          .cursor_pointer()
          .hover(|style| style.opacity(0.7))
          .on_click(cx.listener(|this, _, _, cx| {
            this.sidebar_open = !this.sidebar_open;
            cx.notify();
          }))
          .child(text!("\u{2630}")),
      )
      .child(title(text!(title_text)))
      .child(btn_icon_close())
  }

  fn page_viewport_ui(&self, theme: &Theme, cx: &Context<'_, Self>) -> Stateful<Div> {
    div()
      .id("page-viewport")
      .flex_1()
      .min_w_0()
      .h_full()
      .flex()
      .flex_col()
      .gap_3()
      .p_4()
      .overflow_y_scroll()
      .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, _| {
        this.swipe_x = 0.;
        this.swipe_last = None;
      }))
      .on_drag(PageSwipe, |_, _, _, cx| cx.new(|_| DragGhost))
      .on_drag_move(cx.listener(Self::on_swipe))
      .child(self.page_content_ui(theme, cx))
  }

  fn page_content_ui(&self, theme: &Theme, cx: &Context<'_, Self>) -> AnyElement {
    if let Some(stream) = &self.stream {
      return stream_card(theme, stream).into_any_element();
    }

    let session_q = cx.global::<EngineQuery<SessionV1>>().curr();
    let Some(session) = session_q.value() else {
      return if session_q.error().is_some() {
        text!("Failed to load session").into_any_element()
      } else {
        loading_text("session-loading", "Loading session ...").into_any_element()
      };
    };

    // Fresh session — there are no pages to fetch yet.
    if session.page_count == 0 {
      return text!("Send your first prompt to begin the story ...").into_any_element();
    }

    let page_q = cx.global::<PageQuery>().curr();
    let Some(page) = page_q.value() else {
      return if page_q.error().is_some() {
        text!("Failed to load page").into_any_element()
      } else {
        loading_text("page-loading", "Loading page ...").into_any_element()
      };
    };

    if page_is_empty(page) {
      text!("Send your first prompt to begin the story ...").into_any_element()
    } else {
      page_card(theme, page).into_any_element()
    }
  }

  fn status_row_ui(&self, theme: &Theme, cx: &Context<'_, Self>) -> Div {
    let count = self.session_page_count(cx);
    let page_index = cx.global::<PageQuery>().page_index();
    let streaming = self.stream.is_some();
    let show_indicator = !streaming && count.is_some_and(|count| count > 0);
    let show_jump = !streaming && count.is_some_and(|count| count > 0 && page_index + 1 < count);

    div()
      .w_full()
      .flex()
      .flex_row()
      .justify_center()
      .items_center()
      .gap_3()
      .children(show_indicator.then(|| {
        div()
          .text_sm()
          .text_color(theme.label)
          .child(text!(format!("{}/{}", page_index + 1, count.unwrap())))
      }))
      .children(show_jump.then(|| {
        div()
          .id("jump-to-end")
          .px_3()
          .py_1()
          .text_sm()
          .border_1()
          .border_color(theme.border)
          .rounded_full()
          .cursor_pointer()
          .hover(|style| style.border_color(theme.label).bg(theme.text.alpha(0.1)))
          .on_click(cx.listener(|this, _, _, cx| this.jump_to_end(cx)))
          .child(text!("Jump to end \u{2193}"))
      }))
  }

  fn chat_bar_ui(&self, theme: &Theme, cx: &Context<'_, Self>) -> Div {
    let can_send = self.stream.is_none()
      && self.session_page_count(cx).is_some()
      && !cx.read_entity(&self.input, |input, _| input.value().trim().is_empty());

    let mut send = div()
      .id("btn-send")
      .flex_none()
      .px_4()
      .py_2()
      .rounded_sm()
      .flex()
      .items_center()
      .child(text!("Send"));
    if can_send {
      send = send
        .cursor_pointer()
        .bg(theme.text)
        .text_color(theme.bg)
        .hover(|style| style.bg(theme.text.alpha(0.85)))
        .on_click(cx.listener(|this, _, _, cx| this.submit_input(cx)));
    } else {
      send = send
        .bg(theme.text.alpha(0.1))
        .text_color(Hsla::from(theme.text).opacity(0.9));
    }

    div()
      .w_full()
      .flex()
      .flex_row()
      .items_end()
      .gap_2()
      .p_4()
      .child(self.avatar_ui(theme, cx))
      .child(div().flex_1().min_w_0().child(self.input.clone()))
      .child(send)
  }

  fn avatar_ui(&self, theme: &Theme, cx: &Context<'_, Self>) -> Div {
    let profiles = cx.global::<EngineQuery<Profiles>>().curr();
    let initial = match (self.active_profile_id(cx), profiles.value()) {
      (Some(id), Some(profiles)) => profiles
        .iter()
        .find(|profile| profile.id == id)
        .and_then(|profile| profile.name.chars().next())
        .map(|ch| ch.to_string())
        .unwrap_or_else(|| "?".to_string()),
      _ => "?".to_string(),
    };

    div()
      .relative()
      .flex_none()
      .child(
        div()
          .id("profile-avatar")
          .w(px(40.))
          .h(px(40.))
          .rounded_full()
          .flex()
          .items_center()
          .justify_center()
          .text_lg()
          .bg(theme.text)
          .text_color(theme.bg)
          .cursor_pointer()
          .hover(|style| style.opacity(0.85))
          .on_click(cx.listener(|this, _, _, cx| {
            this.popover_open = !this.popover_open;
            cx.notify();
          }))
          .child(text!(initial)),
      )
      .children(self.popover_open.then(|| self.profile_popover_ui(theme, cx)))
  }

  fn profile_popover_ui(&self, theme: &Theme, cx: &Context<'_, Self>) -> Stateful<Div> {
    let profiles = cx.global::<EngineQuery<Profiles>>().curr();
    let active_id = self.active_profile_id(cx);

    let list_ui = match profiles.value() {
      Some(profiles) if profiles.iter().next().is_some() => div()
        .flex()
        .flex_col()
        .gap_1()
        .children(profiles.iter().map(|profile| {
          let id = profile.id.clone();
          let is_active = active_id.as_deref() == Some(profile.id.as_str());
          div()
            .id(format!("profile-{}", profile.id))
            .px_2()
            .py_1()
            .rounded_sm()
            .flex()
            .flex_row()
            .justify_between()
            .items_center()
            .cursor_pointer()
            .hover(|style| style.bg(theme.text.alpha(0.1)))
            .on_click(cx.listener(move |this, _, _, cx| {
              this.popover_open = false;
              this.switch_profile(&id, cx);
            }))
            .child(text!(profile.name.clone()))
            .children(is_active.then(|| {
              div().text_color(theme.success).child(text!("\u{2713}"))
            }))
        }))
        .into_any_element(),
      _ => div()
        .p_2()
        .text_sm()
        .text_color(theme.label)
        .child(text!("No profiles yet"))
        .into_any_element(),
    };

    div()
      .id("profile-popover")
      .absolute()
      .bottom_full()
      .left_0()
      .mb_2()
      .w(px(224.))
      .max_h(px(280.))
      .overflow_y_scroll()
      .flex()
      .flex_col()
      .gap_1()
      .p_2()
      .bg(theme.bg)
      .border_1()
      .border_color(theme.border)
      .rounded_sm()
      .shadow_lg()
      .child(list_ui)
  }

  fn sidebar_ui(&self, theme: &Theme) -> Div {
    div()
      .w(px(SIDEBAR_WIDTH))
      .flex_none()
      .h_full()
      .border_l_1()
      .border_color(theme.border)
      .p_4()
      .child(
        div()
          .text_sm()
          .text_color(theme.label)
          .child(text!("Game state \u{2014} coming soon")),
      )
  }
}

fn page_is_empty(page: &PageV1) -> bool {
  page.prompt.is_none()
    && page.responses.iter().all(|response| response.content.is_none())
}

fn page_card(theme: &Theme, page: &PageV1) -> Div {
  div()
    .w_full()
    .flex()
    .flex_col()
    .gap_3()
    .children(page.prompt.iter().map(|prompt| {
      div()
        .w_full()
        .p_3()
        .border_1()
        .border_color(theme.border)
        .rounded_sm()
        .text_color(theme.label)
        .child(text!(prompt.clone()))
    }))
    .children(
      page
        .responses
        .iter()
        .filter_map(|response| response.content.as_ref())
        .map(|content| div().w_full().child(text!(content.clone()))),
    )
}

fn stream_card(theme: &Theme, stream: &Stream) -> Div {
  div()
    .w_full()
    .flex()
    .flex_col()
    .gap_3()
    .child(
      div()
        .w_full()
        .p_3()
        .border_1()
        .border_color(theme.border)
        .rounded_sm()
        .text_color(theme.label)
        .child(text!(stream.prompt.clone())),
    )
    .child(if stream.activity {
      loading_text("stream-activity", "The DM is taking action ...").into_any_element()
    } else if stream.text.is_empty() {
      loading_text("stream-thinking", "Thinking ...").into_any_element()
    } else {
      div()
        .w_full()
        .child(text!(stream.text.clone()))
        .into_any_element()
    })
}

impl Render for StoryGameplayScreen {
  fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    let theme = cx.global::<Theme>();

    root(&theme)
      .child(self.top_bar_ui(cx))
      .child(
        div()
          .flex()
          .flex_row()
          .flex_1()
          .min_h_0()
          .w_full()
          .child(self.page_viewport_ui(&theme, cx))
          .children(self.sidebar_open.then(|| self.sidebar_ui(&theme))),
      )
      .child(self.status_row_ui(&theme, cx))
      .child(self.chat_bar_ui(&theme, cx))
  }
}
