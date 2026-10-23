//! The title bar of a window the app draws itself.

use gpui_kit::*;

/// A stretch of title bar: pressing there drags the window, and a double-click does what the
/// system setting says (zoom or minimize). The app owns the title bar (`app_owns_titlebar_drag`):
/// otherwise AppKit takes every double-click in its strip, a button's too.
///
/// A press reaches the front-most element first, so what sits on the row keeps it: a Kit button
/// marks it handled (`prevent_default`), and a title row's icon button or clickable strip takes
/// the pointer (`.occlude()`, as `IconButton::in_title_row` does). A strip laid over a window
/// goes first among its siblings, behind what it covers.
pub(crate) trait TitleDrag: InteractiveElement + Sized {
    fn title_drag(self) -> Self {
        self.window_control_area(WindowControlArea::Drag).on_mouse_down(
            MouseButton::Left,
            |event: &MouseDownEvent, window, _| {
                if window.default_prevented() {
                    return;
                }
                if event.click_count == 2 {
                    window.titlebar_double_click();
                } else {
                    window.start_window_move();
                }
            },
        )
    }
}

impl<E: InteractiveElement> TitleDrag for E {}
