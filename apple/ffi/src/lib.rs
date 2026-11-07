//! The seam between the Swift UI experiment and the Rust terminal: a [`Terminal`] on the real
//! engine (`wormhole-term`, started through `wormhole-core`), its screen as packed bytes, its
//! events behind one coalesced wake.
//!
//! This crate is the only `unsafe` boundary the experiment has in Rust, and it contains none that
//! is written by hand: the allowance below is for the scaffolding UniFFI's proc macros generate
//! (`extern "C"` entry points taking raw pointers and buffers). Nothing here names the UI.

// UniFFI's generated scaffolding is `unsafe extern "C"` code; no hand-written `unsafe` exists in
// this crate (`apple/scripts/rules.py` checks that).
#![allow(unsafe_code, reason = "UniFFI's generated scaffolding")]

mod error;
mod events;
mod frame;
mod interaction;
mod isolation;
pub mod settings_dump;
mod terminal;
pub mod theme_dump;
mod theme_json;
mod theme_resolve;
mod themes;
mod workspace;
mod workspace_feed;
mod workspace_types;

pub use error::FfiError;
pub use events::{TermEvent, TerminalListener};
pub use interaction::{
    CellSide, LinkHit, OpenTarget, PointerAction, PointerButton, PointerEvent, RowSpan, SelectionKind,
    WheelStep,
};
pub use terminal::{FrameTiming, KeyPress, Terminal};
pub use themes::{list_user_themes, resolve_theme};
pub use workspace::{TerminalFactsRecord, Workspace, WorkspaceConfig, WorkspaceEvent, WorkspaceListener};

uniffi::setup_scaffolding!();

/// The packed frame's layout version, for the Swift reader to check against its own.
#[uniffi::export]
pub fn frame_format_version() -> u32 {
    frame::VERSION
}

/// The contract's version (`FFI-CONTRACT.md`), for the Swift side to check against its own at start-up:
/// 2 added `Workspace`. A minor addition (a new event or action case) does not change it.
#[uniffi::export]
pub fn contract_version() -> u32 {
    CONTRACT_VERSION
}

/// See [`contract_version`].
pub(crate) const CONTRACT_VERSION: u32 = 2;

/// A call that does nothing, for measuring what crossing the seam costs by itself.
#[uniffi::export]
pub fn ping(value: u64) -> u64 {
    value
}

/// Tests start real terminals, which refuse to run on real data: give the test process the
/// experiment's own data folder, once, before any test thread reads the variable.
#[cfg(test)]
pub(crate) fn isolate_for_tests() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        let dir = wormhole_core::platform::paths::temp().join("Wormhole-swift-ffi-tests");
        // SAFETY: called once, from the first test that needs it, before it starts a thread of its own.
        // Other test threads only read the variable through `isolation::require`, after this `Once`.
        unsafe { std::env::set_var("WORMHOLE_DATA_DIR", dir) };
    });
}

#[cfg(test)]
mod workspace_tests;
