//! Taking another terminal's settings: what the Settings window's Import page and the first-run
//! step share. The scan reads this Mac's real home, so only the two places a person opens it call
//! [`scan_here`]; devctl scans a fixture home through core directly.

use wormhole_core::import::{self, Env, Found, Proposed, Source};
#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;
use wormhole_text::text;

use gpui_kit::App;

use super::update_settings;

/// What an import did, for the line that says so.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Brought {
    /// Settings taken.
    pub brought: usize,
    /// Settings read that were not taken: unticked, or with no Wormhole setting to go in.
    pub left: usize,
}

/// Every terminal with settings on this Mac, the newest first. Reads files: call it on the
/// background executor, and only when a person has opened the page or the step.
pub(crate) fn scan_here() -> Vec<Found> {
    let mut found = Env::from_process().map(|env| import::scan(&env)).unwrap_or_default();
    resolve_fonts(&mut found);
    found
}

/// Ask the system for the family of each PostScript font name a scan found (iTerm2, Terminal.app).
/// A font that isn't installed leaves the proposal for "Doesn't carry over", so the page never
/// shows, and `apply` never sets, a family that was only guessed from a name.
pub(crate) fn resolve_fonts(found: &mut [Found]) {
    for terminal in found {
        terminal.proposal.resolve_fonts(crate::platform::fonts::family_of_postscript);
    }
}

/// Bring `chosen` (rows of `found`'s proposal) over: a palette is saved as a theme file in the active
/// profile's folder, and everything is set in one settings update. An error is a sentence of the Import page's words.
pub(crate) fn apply(cx: &mut App, found: &Found, chosen: &[Proposed]) -> Result<Brought, String> {
    let saved = import::save_themes(&crate::active_profile::paths().themes_dir(), chosen)
        .map_err(|why| settings_import_failed(&why))?;
    update_settings(cx, |settings| import::apply(settings, &found.proposal, chosen, &saved));
    let proposal = &found.proposal;
    Ok(Brought {
        brought: chosen.len(),
        left: proposal.values.len().saturating_sub(chosen.len()) + proposal.skipped.len(),
    })
}

/// A terminal's name as the interface writes it.
pub(crate) fn terminal_name(source: Source) -> &'static str {
    text(match source {
        Source::Ghostty => SETTINGS_IMPORT_TERMINAL_GHOSTTY,
        Source::ITerm2 => SETTINGS_IMPORT_TERMINAL_ITERM2,
        Source::TerminalApp => SETTINGS_IMPORT_TERMINAL_TERMINAL_APP,
        Source::Alacritty => SETTINGS_IMPORT_TERMINAL_ALACRITTY,
        Source::Kitty => SETTINGS_IMPORT_TERMINAL_KITTY,
        Source::WezTerm => SETTINGS_IMPORT_TERMINAL_WEZTERM,
        Source::Warp => SETTINGS_IMPORT_TERMINAL_WARP,
        Source::Zed => SETTINGS_IMPORT_TERMINAL_ZED,
        Source::VsCode => SETTINGS_IMPORT_TERMINAL_VSCODE,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_terminal_has_a_name() {
        for source in Source::ALL {
            assert!(!terminal_name(source).is_empty(), "{source:?}");
        }
    }
}
