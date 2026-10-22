//! Wormhole's UI-independent core.
//!
//! Everything here builds and tests without GPUI: the terminal engine
//! (PTY + screen + key/mouse encoding), git access, settings and
//! themes, file icon themes, persistence, coding-agent detection and status hooks, and
//! the self-updater.
//!
//! Its modules are grouped by domain, lowest first; a domain uses only the ones
//! before it (`scripts/structure.json`'s `layers` checks it):
//! `platform`, `storage`, `keys`; `theme`, `agents`, `update`; `view`, `project`; `config`; `git`, `notify`;
//! `ipc`; `terminal`; `onboarding`.

pub mod agents;
pub mod config;
pub mod git;
pub mod ipc;
pub mod keys;
pub mod notify;
pub mod onboarding;
pub mod platform;
pub mod project;
pub mod storage;
pub mod terminal;
pub mod theme;
pub mod update;
pub mod view;
