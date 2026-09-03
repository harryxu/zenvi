use gpui::prelude::*;
use gpui::*;
use std::time::Instant;

const TOTAL_LINES: usize = 1000;
const LINE_HEIGHT_PX: f32 = 20.0;
const SCROLLBAR_WIDTH: f32 = 14.0;

struct ScrollBenchView {
    lines: Vec<String>,
    shaped_lines: Vec<Option<ShapedLine>>,
    scroll_top: f32, // in pixels
    is_dragging_scrollbar: bool,
    drag_start_y: f32,
    drag_start_scroll_top: f32,
    total_height: f32,
    fps: f32,
    frame_count: u32,
    fps_timer: Instant,
    use_canvas_fast_path: bool,
}

impl ScrollBenchView {
    fn new(_cx: &mut Context<Self>) -> Self {
        let mut lines = Vec::with_capacity(TOTAL_LINES);
        for i in 1..=TOTAL_LINES {
            let line = match i % 10 {
                0 => format!("{:>4} | pub fn process_data_batch_{}(ctx: &mut Context<Self>, buffer: &[u8; 4096]) -> Result<ProcessedOutput, CustomError> {{", i, i),
                1 => format!("{:>4} |     let mut accumulator = Vec::with_capacity(buffer.len() * 2);", i),
                2 => format!("{:>4} |     for (idx, byte) in buffer.iter().enumerate() {{", i),
                3 => format!("{:>4} |         if *byte == 0x00 || *byte == 0xFF {{ continue; }}", i),
                4 => format!("{:>4} |         let transformed = byte.wrapping_mul(31).rotate_left(3);", i),
                5 => format!("{:>4} |         accumulator.push(transformed ^ (idx as u8));", i),
                6 => format!("{:>4} |     }}", i),
                7 => format!("{:>4} |     log::trace!(\"Batch {} processed successfully with {{}} bytes\", accumulator.len());", i, i),
                8 => format!("{:>4} |     Ok(ProcessedOutput::from_raw_bytes(accumulator))", i),
                _ => format!("{:>4} | }}", i),
            };
            lines.push(line);
        }

        let total_height = TOTAL_LINES as f32 * LINE_HEIGHT_PX;
        let shaped_lines = vec![None; TOTAL_LINES];

        Self {
            lines,
            shaped_lines,
            scroll_top: 0.0,
            is_dragging_scrollbar: false,
            drag_start_y: 0.0,
            drag_start_scroll_top: 0.0,
            total_height,
            fps: 60.0,
            frame_count: 0,
            fps_timer: Instant::now(),
            use_canvas_fast_path: true, // Default to ultra-fast GPU canvas path!
        }
    }
}

impl Render for ScrollBenchView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Track FPS
        self.frame_count += 1;
        let elapsed = self.fps_timer.elapsed().as_secs_f32();
        if elapsed >= 0.5 {
            self.fps = self.frame_count as f32 / elapsed;
            self.frame_count = 0;
            self.fps_timer = Instant::now();
        }

        let viewport = window.viewport_size();
        let viewport_h: f32 = viewport.height.into();

        let title_h = 36.0;
        let content_h = (viewport_h - title_h).max(10.0);
        let max_scroll = (self.total_height - content_h).max(0.0);
        self.scroll_top = self.scroll_top.clamp(0.0, max_scroll);

        // Calculate visible line range (virtualized window)
        let first_visible_line = (self.scroll_top / LINE_HEIGHT_PX).floor() as usize;
        let visible_count = ((content_h / LINE_HEIGHT_PX).ceil() as usize) + 2;
        let last_visible_line = (first_visible_line + visible_count).min(TOTAL_LINES);

        // Subpixel offset for smooth scrolling within a line
        let subpixel_y = -(self.scroll_top % LINE_HEIGHT_PX);

        let default_style = TextStyle {
            font_size: px(14.0).into(),
            line_height: px(LINE_HEIGHT_PX).into(),
            color: rgb(0xcccccc).into(),
            white_space: WhiteSpace::Nowrap,
            ..Default::default()
        };

        // Scrollbar calculations
        let thumb_ratio = (content_h / self.total_height).clamp(0.05, 1.0);
        let thumb_h = (content_h * thumb_ratio).max(24.0);
        let max_thumb_travel = content_h - thumb_h;
        let thumb_top = if max_scroll > 0.0 {
            (self.scroll_top / max_scroll) * max_thumb_travel
        } else {
            0.0
        };

        let mode_text = if self.use_canvas_fast_path {
            "MODE: Pre-Shaped GPU Canvas (Fast Path)"
        } else {
            "MODE: Uncached StyledText DOM (Slow Path)"
        };

        let content_element = if self.use_canvas_fast_path {
            // Fast Path: Lazily shape only newly visible lines once, then paint via GPU Canvas
            for line_idx in first_visible_line..last_visible_line {
                if self.shaped_lines[line_idx].is_none() {
                    let text = SharedString::from(self.lines[line_idx].clone());
                    let run = TextRun {
                        len: text.len(),
                        font: default_style.font(),
                        color: rgb(0xcccccc).into(),
                        background_color: None,
                        underline: None,
                        strikethrough: None,
                    };
                    let shaped = window.text_system().shape_line(
                        text,
                        px(14.0),
                        &[run],
                        None,
                    );
                    self.shaped_lines[line_idx] = Some(shaped);
                }
            }

            let shaped_lines = self.shaped_lines.clone();
            canvas(
                move |_bounds, _window, _cx| {
                    (first_visible_line, last_visible_line, subpixel_y)
                },
                move |bounds, (start, end, subpixel), window, cx| {
                    for (i, line_idx) in (start..end).enumerate() {
                        if let Some(Some(shaped)) = shaped_lines.get(line_idx) {
                            let y = bounds.top() + px(subpixel + i as f32 * LINE_HEIGHT_PX);
                            let origin = Point::new(bounds.left() + px(8.0), y);
                            let _ = shaped.paint(origin, px(LINE_HEIGHT_PX), window, cx);
                        }
                    }
                },
            )
            .size_full()
            .into_any_element()
        } else {
            // Slow Path: Re-creating StyledText DOM elements every frame (Zenvi's original way)
            let mut row_elements = Vec::with_capacity(visible_count);
            for line_idx in first_visible_line..last_visible_line {
                let line_text = &self.lines[line_idx];
                row_elements.push(
                    div()
                        .h(px(LINE_HEIGHT_PX))
                        .w_full()
                        .child(StyledText::new(line_text.clone()).with_default_highlights(&default_style, Vec::new())),
                );
            }
            div()
                .absolute()
                .top(px(subpixel_y))
                .left(px(8.0))
                .right(px(SCROLLBAR_WIDTH + 8.0))
                .flex()
                .flex_col()
                .children(row_elements)
                .into_any_element()
        };

        div()
            .size_full()
            .bg(rgb(0x1a1b26))
            .flex()
            .flex_col()
            // Titlebar with FPS and Mode Toggle
            .child(
                div()
                    .h(px(title_h))
                    .w_full()
                    .bg(rgb(0x16161e))
                    .border_b_1()
                    .border_color(rgb(0x2f3549))
                    .flex()
                    .items_center()
                    .px(px(16.0))
                    .justify_between()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(12.0))
                            .child(
                                div()
                                    .text_color(rgb(0x7aa2f7))
                                    .text_size(px(13.0))
                                    .font_weight(FontWeight::BOLD)
                                    .child(format!(
                                        "FPS: {:.1} | Scroll: {:.0}px / {:.0}px",
                                        self.fps, self.scroll_top, max_scroll
                                    )),
                            )
                            .child(
                                div()
                                    .text_color(if self.use_canvas_fast_path { rgb(0x9ece6a) } else { rgb(0xf7768e) })
                                    .text_size(px(12.0))
                                    .font_weight(FontWeight::BOLD)
                                    .child(mode_text),
                            ),
                    )
                    .child(
                        div()
                            .px(px(8.0))
                            .py(px(2.0))
                            .rounded(px(4.0))
                            .bg(rgb(0x24283b))
                            .hover(|s| s.bg(rgb(0x3b4261)))
                            .text_color(rgb(0xc0caf5))
                            .text_size(px(11.0))
                            .child("Click to Switch Mode")
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _event: &MouseDownEvent, _window, cx| {
                                    this.use_canvas_fast_path = !this.use_canvas_fast_path;
                                    cx.notify();
                                }),
                            ),
                    ),
            )
            // Content area
            .child(
                div()
                    .flex_1()
                    .w_full()
                    .relative()
                    .overflow_hidden()
                    // Mouse wheel scrolling
                    .on_scroll_wheel(cx.listener(move |this, event: &ScrollWheelEvent, _window, cx| {
                        let delta_y = match event.delta {
                            ScrollDelta::Pixels(p) => {
                                let y: f32 = p.y.into();
                                -y
                            }
                            ScrollDelta::Lines(l) => {
                                let y: f32 = l.y.into();
                                -y * LINE_HEIGHT_PX * 3.0
                            }
                        };
                        this.scroll_top = (this.scroll_top + delta_y).clamp(0.0, max_scroll);
                        cx.notify();
                    }))
                    // Mouse drag handling on container
                    .on_mouse_up(
                        MouseButton::Left,
                        cx.listener(|this, _event: &MouseUpEvent, _window, _cx| {
                            this.is_dragging_scrollbar = false;
                        }),
                    )
                    .on_mouse_move(cx.listener(move |this, event: &MouseMoveEvent, _window, cx| {
                        if this.is_dragging_scrollbar && max_thumb_travel > 0.0 {
                            let mouse_y: f32 = event.position.y.into();
                            let delta_y = mouse_y - this.drag_start_y;
                            let scroll_delta = (delta_y / max_thumb_travel) * max_scroll;
                            this.scroll_top = (this.drag_start_scroll_top + scroll_delta).clamp(0.0, max_scroll);
                            cx.notify();
                        }
                    }))
                    .child(content_element)
                    // Scrollbar track
                    .child(
                        div()
                            .absolute()
                            .top(px(0.0))
                            .right(px(0.0))
                            .w(px(SCROLLBAR_WIDTH))
                            .h_full()
                            .bg(rgb(0x13141c))
                            // Scrollbar Thumb
                            .child(
                                div()
                                    .absolute()
                                    .top(px(thumb_top))
                                    .left(px(2.0))
                                    .right(px(2.0))
                                    .h(px(thumb_h))
                                    .rounded(px(4.0))
                                    .bg(rgb(0x414868))
                                    .hover(|s| s.bg(rgb(0x7aa2f7)))
                                    .on_mouse_down(
                                        MouseButton::Left,
                                        cx.listener(move |this, event: &MouseDownEvent, _window, cx| {
                                            this.is_dragging_scrollbar = true;
                                            this.drag_start_y = event.position.y.into();
                                            this.drag_start_scroll_top = this.scroll_top;
                                            cx.notify();
                                        }),
                                    ),
                            ),
                    ),
            )
    }
}

fn main() {
    env_logger::init();

    let app = Application::new();
    app.run(|cx: &mut App| {
        let mut window_options = WindowOptions::default();
        let bounds = Bounds::new(
            Point::new(px(100.0), px(100.0)),
            Size::new(px(1100.0), px(750.0)),
        );
        window_options.window_bounds = Some(WindowBounds::Windowed(bounds));
        window_options.titlebar = Some(TitlebarOptions {
            title: Some("GPUI Pure Scroll Benchmark (Fast vs Slow)".into()),
            appears_transparent: false,
            traffic_light_position: None,
        });

        cx.open_window(window_options, |_, cx| {
            cx.new(|cx| ScrollBenchView::new(cx))
        })
        .unwrap();
    });
}
