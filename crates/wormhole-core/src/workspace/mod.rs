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
//! Layers inside, lowest first: `facts`, then `status` (a terminal's status machine) and `git_feed`
//! (what git says of its folder), `session`, `action` and `event`, `model` and its `impl`s
//! (`terminals`, `groups`, `drops`, `splits`, `close`, `status_feed`, `git_sync`), then `words` and
//! `snapshot`.

mod action;
mod close;
mod drops;
mod event;
mod extras;
mod facts;
mod fence;
mod git_feed;
mod git_sync;
mod ground;
mod groups;
mod ids;
mod model;
mod moves;
mod open_items;
mod overview;
mod panel;
mod presets;
mod saved;
mod session;
mod snapshot;
mod splits;
mod status;
mod status_feed;
mod terminals;
mod tile;
mod words;

#[cfg(test)]
mod tests;

pub use action::{Action, CardRef, Over, Overlay};
pub use close::{asks_before_closing, session_after_close};
pub use drops::DropPlan;
pub use event::{Event, Refusal, Revisions};
pub use facts::{Failure, Millis, ProgramNotice, TerminalFacts};
pub use git_feed::{
    ChangeRules, GitFacts, GitFeed, GitJob, PROBE_CONFIRM, Repo, Sight, Ticket, WATCHED_READ, Wanted,
};
pub use ground::{DiskGround, Ground};
pub use ids::IdCounter;
pub use model::{ModelConfig, WorkspaceModel};
pub use moves::Departing;
pub use open_items::open_items;
pub use overview::SplitOverview;
pub use saved::SavedLayout;
pub use snapshot::{
    AgentInfo, ChromeSnapshot, ColumnLimits, DividerInfo, EdgeDrag, GroupHead, LayoutInfo, MemberTile,
    PaneRect, PanelTab, PanesSnapshot, PathFolder, PathStep, PresetInfo, SidebarRow, SidebarSection,
    SidebarSnapshot, SplitInfo, TerminalCounts, TerminalSummary, TerminalsSnapshot, TitleSlot, TitleSnapshot,
    TitleTerminal,
};
pub use status::{Ask, Context, ForegroundLook, Front, StatusEvent, StatusMachine, status_of};
pub use tile::{TileGlyph, glyph_of};
pub use words::{SecondLine, Tone};
