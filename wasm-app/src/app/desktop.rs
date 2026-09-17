//! A tiny window manager: draggable, closable windows on a desktop
//! background with a taskbar, all driven purely from Rust state and
//! rendered as GPU quads via [`crate::painter::Painter`].

use super::geometry::Rect;
use super::painter::{Color, Painter};

pub const TITLE_BAR_HEIGHT: f32 = 28.0;
pub const TASKBAR_HEIGHT: f32 = 32.0;

const DESKTOP_BG: Color = Color::rgb(0x0d, 0x11, 0x17);
const TASKBAR_BG: Color = Color::rgb(0x16, 0x1b, 0x22);
const WINDOW_BG: Color = Color::rgb(0x16, 0x1b, 0x22);
const TITLE_BAR_BG: Color = Color::rgb(0x21, 0x26, 0x2d);
const TITLE_BAR_FOCUSED_BG: Color = Color::rgb(0x1f, 0x6f, 0xeb);
const BORDER: Color = Color::rgb(0x30, 0x36, 0x3d);
const TEXT: Color = Color::rgb(0xe6, 0xed, 0xf3);
const CLOSE_BTN: Color = Color::rgb(0xf8, 0x51, 0x49);

/// A pluggable "program" that runs inside a desktop window. Apps only
/// know about their own content area in local pixel coordinates; the
/// window manager takes care of chrome, dragging and focus.
pub trait App {
    fn title(&self) -> &str;

    /// Called once per animation frame before rendering, with the
    /// current `performance.now()` timestamp in milliseconds. Used e.g.
    /// by [`crate::apps::comet_app::CometApp`] to time the AI's turn.
    fn update(&mut self, now_ms: f64);

    fn render(&mut self, painter: &mut Painter, content: Rect);

    /// `pos` is in local content-area coordinates.
    fn on_pointer_down(&mut self, pos: (f32, f32));
}

struct Window {
    app: Box<dyn App>,
    rect: Rect,
}

enum Drag {
    None,
    MoveWindow { index: usize, offset: (f32, f32) },
}

pub struct Desktop {
    windows: Vec<Window>,
    focused: Option<usize>,
    drag: Drag,
    size: (f32, f32),
}

impl Desktop {
    pub fn new(size: (f32, f32)) -> Self {
        Self { windows: Vec::new(), focused: None, drag: Drag::None, size }
    }

    pub fn set_size(&mut self, size: (f32, f32)) {
        self.size = size;
    }

    pub fn open_window(&mut self, app: Box<dyn App>, rect: Rect) {
        self.windows.push(Window { app, rect });
        self.focused = Some(self.windows.len() - 1);
    }

    pub fn update(&mut self, now_ms: f64) {
        for window in &mut self.windows {
            window.app.update(now_ms);
        }
    }

    fn title_bar_rect(&self, index: usize) -> Rect {
        let w = self.windows[index].rect;
        Rect::new(w.x, w.y, w.w, TITLE_BAR_HEIGHT)
    }

    fn close_button_rect(&self, index: usize) -> Rect {
        let bar = self.title_bar_rect(index);
        let size = TITLE_BAR_HEIGHT - 8.0;
        Rect::new(bar.x + bar.w - size - 6.0, bar.y + 4.0, size, size)
    }

    fn content_rect(&self, index: usize) -> Rect {
        let w = self.windows[index].rect;
        Rect::new(w.x, w.y + TITLE_BAR_HEIGHT, w.w, w.h - TITLE_BAR_HEIGHT)
    }

    pub fn render(&mut self, painter: &mut Painter) {
        painter.rect(Rect::new(0.0, 0.0, self.size.0, self.size.1), DESKTOP_BG);

        for index in 0..self.windows.len() {
            let focused = self.focused == Some(index);
            let win_rect = self.windows[index].rect;
            painter.rect(win_rect, WINDOW_BG);

            let bar = self.title_bar_rect(index);
            painter.rect(bar, if focused { TITLE_BAR_FOCUSED_BG } else { TITLE_BAR_BG });
            painter.text(bar.x + 8.0, bar.y + 8.0, 2.0, TEXT, self.windows[index].app.title());

            let close = self.close_button_rect(index);
            painter.rect(close, CLOSE_BTN);

            painter.rect_outline(win_rect, 1.0, BORDER);

            let content = self.content_rect(index);
            self.windows[index].app.render(painter, content);
        }

        let taskbar = Rect::new(0.0, self.size.1 - TASKBAR_HEIGHT, self.size.0, TASKBAR_HEIGHT);
        painter.rect(taskbar, TASKBAR_BG);
        let mut x = taskbar.x + 8.0;
        for window in &self.windows {
            let label_w = Painter::text_width(window.app.title(), 2.0) + 16.0;
            painter.rect(
                Rect::new(x, taskbar.y + 4.0, label_w, TASKBAR_HEIGHT - 8.0),
                TITLE_BAR_BG,
            );
            painter.text(x + 8.0, taskbar.y + 12.0, 2.0, TEXT, window.app.title());
            x += label_w + 8.0;
        }
    }

    pub fn pointer_down(&mut self, x: f32, y: f32) {
        // Search topmost-first so overlapping windows behave sensibly.
        for index in (0..self.windows.len()).rev() {
            let win_rect = self.windows[index].rect;
            if !win_rect.contains(x, y) {
                continue;
            }
            self.focused = Some(index);

            if self.close_button_rect(index).contains(x, y) {
                self.windows.remove(index);
                self.focused = self.windows.len().checked_sub(1);
                return;
            }

            if self.title_bar_rect(index).contains(x, y) {
                self.drag =
                    Drag::MoveWindow { index, offset: (x - win_rect.x, y - win_rect.y) };
                return;
            }

            let content = self.content_rect(index);
            if content.contains(x, y) {
                self.windows[index]
                    .app
                    .on_pointer_down((x - content.x, y - content.y));
            }
            return;
        }
    }

    pub fn pointer_move(&mut self, x: f32, y: f32) {
        if let Drag::MoveWindow { index, offset } = self.drag {
            if let Some(window) = self.windows.get_mut(index) {
                window.rect.x = (x - offset.0).max(0.0);
                window.rect.y = (y - offset.1).max(0.0);
            }
        }
    }

    pub fn pointer_up(&mut self) {
        self.drag = Drag::None;
    }
}
