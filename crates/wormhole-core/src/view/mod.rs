//! What the app's views show, as data and rules without GPUI: the sidebar's list (`sidebar`), the
//! columns, title row and status bar (`layout`), split panes (`panes`), where focus goes between
//! areas (`focus`), the command palette's matching (`search`), VoiceOver's words (`spoken`), the
//! panel's tabs (`panel`), drag and drop order (`reorder`), shortened text (`elide`), the panels'
//! caps and counts (`limits`) and the title row's extras (`title_extras`).
//!
//! It uses only `theme` below it: never settings, git, the store or the running app. What those know
//! comes to it as values.

pub mod elide;
pub mod focus;
pub mod layout;
pub mod limits;
pub mod panel;
pub mod panes;
pub mod reorder;
pub mod search;
pub mod sidebar;
pub mod spoken;
pub mod title_extras;
