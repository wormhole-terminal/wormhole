//! Wormhole's UI-independent core.
//!
//! Everything here builds and tests without GPUI: git access, settings and
//! themes, file icon themes, persistence, coding-agent status hooks, starting a terminal's
//! program, and the self-updater. The terminal itself (the PTY, the screen, key and mouse
//! encoding, agent detection) is `wormhole_term`, its own crate so the session host can link it
//! without the rest of core.
//!
//! Its modules are grouped by domain, lowest first; a domain uses only the ones
//! before it or in its own tier (`scripts/structure.json`'s `layers` checks it):
//! `platform`, `storage`, `keys`; `theme`, `agents`, `update`; `view`, `project`; `config`; `git`, `notify`;
//! `ipc`; `terminal`; `onboarding`.

#![cfg_attr(test, allow(clippy::disallowed_methods, reason = "tests build their own files and processes"))]
#![allow(unreachable_pub, reason = "not yet cleaned (R-13)")]

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
