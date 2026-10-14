//! The cursor: where it is drawn, and the character redrawn inside a block.

use std::time::Duration;

use gpui_kit::{Bounds, Context, Hsla, Pixels, Point, point, px, size};
use wormhole_core::alacritty_terminal::vte::ansi::CursorShape;

use crate::palette::Palette;
use crate::terminal_view::{Status, TerminalView};

/// Each phase of a blinking cursor; xterm's and macOS's own text cursor rate.
const BLINK_PHASE: Duration = Duration::from_millis(530);

impl TerminalView {
    /// After each frame: blink while a focused terminal's program asks for it (and Reduce
    /// Motion is off). A timer runs only then, so a steady cursor costs no wakeups.
    pub(super) fn blink_wanted(&mut self, wanted: bool, cx: &mut Context<Self>) {
        self.wants_blink = wanted;
        if !wanted || self.cursor_blink.is_some() {
            return;
        }
        self.cursor_blink = Some(cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(BLINK_PHASE).await;
                let blinking = this.update(cx, |this, cx| {
                    if !this.wants_blink {
                        this.cursor_blink = None;
                        if std::mem::take(&mut this.cursor_off) {
                            this.notify_redraw(cx);
                        }
                        return false;
                    }
                    let held = std::mem::take(&mut this.blink_held);
                    let off = !this.cursor_off && !held;
                    if off != this.cursor_off {
                        this.cursor_off = off;
                        this.notify_redraw(cx);
                    }
                    true
                });
                if !matches!(blinking, Ok(true)) {
                    return;
                }
            }
        }));
    }

    /// Typing shows the cursor at once and keeps it on for a phase, as in any text field.
    pub(in crate::terminal_view) fn hold_cursor(&mut self) {
        self.blink_held = true;
        self.cursor_off = false;
    }
}

/// Whether the cursor is drawn at all: not once the shell has exited, when nothing reads keys
/// there any more ("Process exited" has no cursor above the bar).
pub(super) fn drawn(status: Status) -> bool {
    status != Status::Exited
}

/// The cursor's rectangle in a cell `width` wide at `origin`. A beam or an
/// underline only while focused; otherwise the whole cell (filled when
/// focused, an outline when not).
pub(super) fn cursor_rect(
    shape: CursorShape,
    focused: bool,
    origin: Point<Pixels>,
    width: Pixels,
    line_height: Pixels,
) -> Bounds<Pixels> {
    match (shape, focused) {
        (CursorShape::Beam, true) => Bounds::new(origin, size(px(2.0), line_height)),
        (CursorShape::Underline, true) => {
            Bounds::new(point(origin.x, origin.y + line_height - px(2.0)), size(width, px(2.0)))
        }
        _ => Bounds::new(origin, size(width, line_height)),
    }
}

/// Whether the character under the cursor is redrawn on top of it: only a
/// filled block covers it.
pub(super) fn redraws_text(rect: Bounds<Pixels>, focused: bool) -> bool {
    focused && rect.size.width > px(2.0) && rect.size.height > px(2.0)
}

/// The color of that character: the theme's cursor text, else the cell's
/// background so it reads as cut out of the block.
pub(super) fn text_color(palette: &Palette, cell_background: Hsla) -> Hsla {
    palette.cursor_text.unwrap_or(cell_background)
}

#[cfg(test)]
mod tests {
    use gpui_kit::{point, px};
    use wormhole_core::alacritty_terminal::vte::ansi::CursorShape;
    use wormhole_core::theme;

    use super::{cursor_rect, drawn, redraws_text, text_color};
    use crate::palette::Palette;
    use crate::terminal_view::Status;

    #[test]
    fn an_exited_shell_has_no_cursor() {
        assert!(!drawn(Status::Exited));
        for status in [Status::Idle, Status::Busy, Status::Done, Status::Failed, Status::Attention] {
            assert!(drawn(status), "{status:?}");
        }
    }

    #[test]
    fn beams_and_underlines_only_while_focused() {
        let origin = point(px(10.0), px(20.0));
        let beam = cursor_rect(CursorShape::Beam, true, origin, px(8.0), px(18.0));
        assert_eq!((beam.size.width, beam.size.height), (px(2.0), px(18.0)));
        assert!(!redraws_text(beam, true));
        let under = cursor_rect(CursorShape::Underline, true, origin, px(16.0), px(18.0));
        assert_eq!((under.origin.y, under.size.width), (px(36.0), px(16.0)));
        let unfocused = cursor_rect(CursorShape::Beam, false, origin, px(8.0), px(18.0));
        assert_eq!(unfocused.size.width, px(8.0));
        assert!(!redraws_text(unfocused, false), "an outline doesn't cover the character");
        assert!(redraws_text(cursor_rect(CursorShape::Block, true, origin, px(8.0), px(18.0)), true));
    }

    #[test]
    fn the_character_uses_the_themes_cursor_text() {
        let themes = theme::builtin();
        let with = themes.iter().find(|t| t.cursor_text.is_some()).expect("a theme with cursor text");
        let palette = Palette::from_theme(with);
        let background = palette.ansi[4];
        assert_eq!(text_color(&palette, background), palette.cursor_text.expect("set"));
        let mut without = with.clone();
        without.cursor_text = None;
        assert_eq!(text_color(&Palette::from_theme(&without), background), background);
    }
}
