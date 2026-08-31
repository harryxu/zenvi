use super::{ZenviView, GRID_PADDING_LEFT, TOP_OFFSET};
use gpui::*;

impl EntityInputHandler for ZenviView {
    fn text_for_range(
        &mut self,
        range_utf16: std::ops::Range<usize>,
        actual_range: &mut Option<std::ops::Range<usize>>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<String> {
        if let Some(ref text) = self.marked_text {
            let u16_chars: Vec<u16> = text.encode_utf16().collect();
            let start = range_utf16.start.min(u16_chars.len());
            let end = range_utf16.end.min(u16_chars.len());
            if start <= end {
                *actual_range = Some(start..end);
                return Some(String::from_utf16_lossy(&u16_chars[start..end]));
            }
        }
        None
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        let len = self
            .marked_text
            .as_ref()
            .map(|t| t.encode_utf16().count())
            .unwrap_or(0);
        Some(UTF16Selection {
            range: len..len,
            reversed: false,
        })
    }

    fn marked_text_range(
        &self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<std::ops::Range<usize>> {
        self.marked_text
            .as_ref()
            .map(|t| 0..t.encode_utf16().count())
    }

    fn unmark_text(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {
        self.marked_text = None;
    }

    fn replace_text_in_range(
        &mut self,
        _range_utf16: Option<std::ops::Range<usize>>,
        new_text: &str,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
        let had_marked = self.marked_text.is_some();
        self.marked_text = None;
        if !new_text.is_empty() {
            let is_non_ascii = !new_text.is_ascii();
            let is_multi_char = new_text.chars().count() > 1;

            // Only send text from IME if it was composing (marked_text was present)
            // or if it contains non-ASCII characters (e.g. Chinese characters from direct IME commit) or multi-char strings.
            // Normal ASCII keystrokes are handled directly by `on_key_down` to prevent double-typing across all platforms.
            if had_marked || is_non_ascii || is_multi_char {
                if new_text == "<" {
                    self.session.send_input("<lt>");
                } else {
                    self.session.send_input(new_text);
                }
            }
        }
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        _range_utf16: Option<std::ops::Range<usize>>,
        new_text: &str,
        _new_selected_range_utf16: Option<std::ops::Range<usize>>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
        if new_text.is_empty() {
            self.marked_text = None;
        } else {
            self.marked_text = Some(new_text.to_string());
        }
    }

    fn bounds_for_range(
        &mut self,
        _range_utf16: std::ops::Range<usize>,
        element_bounds: Bounds<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let state = self.session.state.read();
        let (cursor_row, cursor_col) = state
            .grids
            .get(&1)
            .map(|g| (g.cursor_row, g.cursor_col))
            .unwrap_or((0, 0));
        drop(state);

        let x = GRID_PADDING_LEFT + cursor_col as f32 * self.char_width;
        let y = TOP_OFFSET + cursor_row as f32 * f32::from(self.line_height);
        let cursor_bounds = Bounds::new(
            Point::new(
                element_bounds.origin.x + px(x),
                element_bounds.origin.y + px(y),
            ),
            Size::new(px(self.char_width), self.line_height),
        );
        Some(cursor_bounds)
    }

    fn character_index_for_point(
        &mut self,
        _point: Point<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<usize> {
        Some(0)
    }
}
