//! ⌃↩ in a list opens the row's context menu from the keyboard: the menu of the row the keys are
//! on, the one a right-click shows, opened under that row as macOS's lists do under Full Keyboard
//! Access. It is a native menu, which has the keyboard while it is open (arrows, type-to-select,
//! Esc) and gives it back to the list when it closes: nothing here moves the focus. A list whose
//! rows have no menu yet (the panel's) still takes the key, so it never reaches the
//! terminal behind it.

use gpui_kit::*;
use wormhole_core::keys::keymap::{self, ActionId};
use wormhole_core::view::focus::Area;

use crate::style;
use crate::window::Workspace;

/// Whether `keystroke` is the row menu's key.
pub(super) fn is_menu_key(keystroke: &Keystroke) -> bool {
    keymap::list_action(&crate::terminal_view::keystroke_spec(keystroke)) == Some(ActionId::RowMenu)
}

/// Where a menu opened from the keys starts: under the row, in from its edge by the list's inset
/// (a flush row starts at the window's edge, where the menu's corners would be cut off), where a
/// right-click on its lead column would put it.
pub(in crate::window) fn key_menu_origin(row: Bounds<Pixels>) -> Point<Pixels> {
    point(row.left() + px(style::SIDEBAR_INSET), row.bottom())
}

impl Workspace {
    /// ⌃↩ in `area`'s list: open the menu of the row the keys are on, if it has one.
    pub(super) fn row_menu_key(&mut self, area: Area, window: &mut Window, cx: &mut Context<Self>) {
        // The panel's rows have no menu yet; the title strip's is the folder's path menu.
        match area {
            Area::Sidebar => self.sidebar_key_menu(window, cx),
            Area::Tabs => self.open_path_menu(window, cx),
            _ => {}
        }
    }
}
