//! The screens of the terminals, kept each minute (`window/save.rs` `save_scrollback`): what a terminal whose host is
//! gone is drawn from when the app starts again. Sealed by core's store with the profile's key, in a file of its own.

use super::desk::{HostedTerminals, locked};

/// The lines of history kept per terminal (`SCROLLBACK_LINES` in `window/save.rs`).
const SAVED_LINES: usize = 2000;

#[uniffi::export]
impl HostedTerminals {
    /// Save the screen text of every terminal this object made, unless the profile keeps none ("Restore terminal text
    /// after restart") or the terminal's row says not to (core's `forget_text`, which the store honours). Reads the
    /// engine and seals a file: not on the main thread. How many were written.
    pub fn save_texts(&self) -> u32 {
        if !self.keeps_text() {
            return 0;
        }
        let texts: Vec<(i64, String)> = locked(&self.made)
            .iter()
            .filter_map(|(tab, weak)| {
                let terminal = weak.upgrade()?;
                terminal.with_pty(|pty| pty.scrollback().text(SAVED_LINES)).flatten().map(|text| (*tab, text))
            })
            .collect();
        match self.registry.save_texts(&texts) {
            Ok(written) => written,
            Err(err) => {
                tracing::warn!("saving the terminals' text failed: {err}");
                0
            }
        }
    }
}

impl HostedTerminals {
    /// Whether the default profile keeps terminal text: its setting. (What turning it off erases is core's.)
    pub(super) fn keeps_text(&self) -> bool {
        wormhole_core::config::Config::load_for(&self.profile).settings.restore_terminal_text
    }
}
