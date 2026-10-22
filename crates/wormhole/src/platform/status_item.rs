//! The status item ("menu extra") as the system draws it: one per window, whose icon says whether
//! a terminal needs you and whose menu lists the terminals by status. What it shows is the app's
//! (`menubar::model`, `workspace::menu_bar`); this is only the drawing. Everything here runs on
//! the main thread.

pub use super::os::status_item::MenuBar;
#[cfg(feature = "devctl")]
pub use super::os::status_item::save_icons;
