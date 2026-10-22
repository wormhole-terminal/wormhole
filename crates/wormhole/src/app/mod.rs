//! The app root: what starts the app and steers it from above its windows. It launches (`launch`),
//! runs the app's tick and what arrives between ticks (`tick`), binds the app-wide actions and
//! handles them (`handlers`), builds the menu bar (`menus`), answers the control socket (`control`),
//! takes the files macOS hands over (`default_terminal`), keeps a terminal's saved text (`saved_text`)
//! and logs (`logging`).
//!
//! The top of the order: it may use every surface, and nothing below it may call into it.

pub(crate) mod control;
pub(crate) mod default_terminal;
pub(crate) mod handlers;
pub(crate) mod launch;
pub(crate) mod logging;
pub(crate) mod menus;
mod saved_text;
#[cfg(test)]
mod saved_text_tests;
mod tick;
