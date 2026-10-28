//! What the app's views show, as data and rules without GPUI: the sidebar's list (`sidebar`), the
//! columns, title row and status bar (`layout`), the Files tab's tree and path menu (`file_tree`), split panes
//! (`panes`), where focus goes between areas (`focus`), the command palette's matching (`search`), VoiceOver's words (`spoken`), the
//! panel's tabs (`panel`), drag and drop order (`reorder`), shortened text (`elide`), the panels'
//! caps and counts (`limits`), which animation each motion runs (`motion`), the notes and next steps
//! of a terminal (`notes`), the notice chip's rules (`notice`), where a terminal torn off the window lands
//! (`tear_off`), the title row's extras (`title_extras`) and the folder picker at the prompt
//! (`folder_picker`).
//!
//! It uses only `theme` below it (and the plain values `storage` hands over, a folder's listing): never
//! settings, git, the store or the running app. What those know comes to it as values.

pub mod elide;
pub mod file_tree;
pub mod focus;
pub mod folder_picker;
pub mod layout;
pub mod limits;
pub mod motion;
pub mod notes;
pub mod notice;
pub mod panel;
pub mod panes;
pub mod reorder;
pub mod search;
pub mod sidebar;
pub mod spoken;
pub mod tear_off;
pub mod title_extras;
