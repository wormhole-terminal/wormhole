//! ⌃↩ in a list opens the row's context menu from the keyboard: the menu of the row the keys are
//! on, the one a right-click shows, opened under that row as macOS's lists do under Full Keyboard
//! Access. It is a native menu, which has the keyboard while it is open (arrows, type-to-select,
//! Esc) and gives it back to the list when it closes: nothing here moves the focus. A list whose
//! rows have no menu yet (the panel's) still takes the key, so it never reaches the
//! terminal behind it.

use gpui_kit::*;
use wormhole_core::view::focus::Area;
use wormhole_core::view::layout::title_parts::TitlePart;

use crate::settings::settings_in;
use crate::window::Workspace;

impl Workspace {
    /// ⌃↩ in `area`'s list: open the menu of the row the keys are on, if it has one.
    pub(super) fn row_menu_key(&mut self, area: Area, window: &mut Window, cx: &mut Context<Self>) {
        // The panel's rows have no menu yet; the title strip's is the folder's path menu, or the
        // row's own where the row doesn't show the folder (under the strip, where the probe is then).
        if area == Area::Tabs {
            let folder = self.active_session().pane.summary.cwd.is_some()
                && settings_in(&self.profile, cx).title_shows(TitlePart::Folder);
            match folder {
                true => self.open_path_menu(window, cx),
                false => self.open_title_menu(self.path_anchor.below(), window, cx),
            }
        }
    }
}
