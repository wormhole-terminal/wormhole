//! The status item ("menu extra") as the system draws it: one per window, whose icon says whether
//! a terminal needs input and whose menu lists the terminals by status. What it shows is the app's
//! (`menubar::model`, `status_item`); this is only the drawing. Everything here runs on
//! the main thread.

use smol::channel::Sender;

use super::background_mode;
pub use super::os::status_item::MenuBar;
use crate::menubar::MenuCommand;

/// Put the status item in the menu bar; commands arrive on `tx` from the main thread as the user
/// picks items. `None` where there is none to put there, and in background mode: a test instance
/// keeps out of the menu bar (devctl's `menubar` still shows the menu it would have).
pub fn install(tx: Sender<MenuCommand>) -> Option<MenuBar> {
    if background_mode::on() {
        return None;
    }
    MenuBar::install(tx)
}
#[cfg(feature = "devctl")]
pub use super::os::status_item::save_icons;
