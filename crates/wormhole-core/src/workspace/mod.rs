//! The window's state with no window: the terminals of one window, their groups, pins and splits,
//! what the sidebar lists of them and what the person did (`Action`), as a sans-IO state machine.
//!
//! A shell (the GPUI app today, a Swift app through `apple/ffi`) feeds it what it learns of each
//! terminal (`TerminalFacts`), tells it what a person did (`WorkspaceModel::apply`) and pulls what to
//! draw (`snapshot`: plain records, revisioned) when told that something changed (`Event`). The model
//! has no clock, thread, I/O or UI type of its own: a time is a [`Millis`] the shell reads, a new
//! terminal is an `Event::Opened` the shell answers by starting a program, and every rule is core's:
//! `view::sidebar` (listing, pins, groups, drops, words), `view::panes` (splits) and
//! `config::tab` (the panel's tabs). This is the first slice of `swift-headless-workspace.md`: the
//! terminal list and its splits. Settings, features, focus areas, saving and the app's other windows
//! join it as they are extracted.
//!
//! Layers inside, lowest first: `facts`, `session`, `action` and `event`, `model` and its `impl`s
//! (`terminals`, `groups`, `drops`, `splits`, `close`), then `words` and `snapshot`.

mod action;
mod close;
mod drops;
mod event;
mod facts;
mod groups;
mod model;
mod session;
mod snapshot;
mod splits;
mod terminals;
mod words;

#[cfg(test)]
mod tests;

pub use action::{Action, CardRef, Over, Overlay};
pub use close::session_after_close;
pub use drops::DropPlan;
pub use event::{Event, Refusal, Revisions};
pub use facts::{Failure, Millis, TerminalFacts};
pub use model::{ModelConfig, WorkspaceModel};
pub use snapshot::{
    AgentInfo, ChromeSnapshot, GroupHead, PaneRect, PanesSnapshot, SidebarRow, SidebarSection,
    SidebarSnapshot, SplitInfo,
};
pub use words::{SecondLine, Tone};
