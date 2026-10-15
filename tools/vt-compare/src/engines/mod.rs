//! The engines this crate adds to the one in `wormhole-vt-bench` (Wormhole's own): alacritty_terminal,
//! vt100 and WezTerm's terminal model (always), Ghostty (when `fetch-ghostty.sh` has built
//! libghostty-vt).
//!
//! [`for_each_engine`] runs a visitor over all of them, alacritty first (the baseline the others are
//! compared to), then the bench crate's own, so a new engine registered there is compared here
//! without a change.

mod alacritty;
#[cfg(has_ghostty)]
pub mod ghostty;
mod vt100;
mod wezterm;

pub use alacritty::AlacrittyEngine;
#[cfg(has_ghostty)]
pub use ghostty::GhosttyEngine;
pub use vt100::Vt100Engine;
pub use wezterm::WeztermEngine;
use wormhole_vt_bench::Engine;
use wormhole_vt_bench::engines::EngineVisitor;

/// Where a line is read: the screen (row 0 the top) or the history (line 0 the oldest). The adapters
/// that keep the two apart take one in place of a flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Region {
    Screen,
    History,
}

/// The names of every engine that can run here, in the order [`for_each_engine`] visits them.
pub fn engine_names() -> Vec<&'static str> {
    let mut names: Vec<&'static str> = vec![AlacrittyEngine::NAME];
    names.extend(wormhole_vt_bench::engines::ENGINE_NAMES);
    names.push(WeztermEngine::NAME);
    names.push(Vt100Engine::NAME);
    #[cfg(has_ghostty)]
    names.push(GhosttyEngine::NAME);
    names
}

/// What the engine called `name` is, for a report: the library and the version or commit built.
pub fn engine_info(name: &str) -> String {
    match name {
        "alacritty" => format!("alacritty_terminal {}", env!("ALACRITTY_TERMINAL_VERSION")),
        "wezterm" => format!("wezterm-term @{} (WezTerm's terminal model)", env!("WEZTERM_TERM_REV")),
        "vt100" => format!("vt100 {}", env!("VT100_VERSION")),
        "ghostty" => format!("libghostty-vt @{} (Ghostty's terminal)", env!("GHOSTTY_REV")),
        other => other.to_string(),
    }
}

/// Engines this crate knows but could not build, and what to do about it.
pub fn unavailable() -> Vec<(&'static str, &'static str)> {
    let mut out = Vec::new();
    if cfg!(not(has_ghostty)) {
        out.push((
            "ghostty",
            "libghostty-vt is not built: run tools/vt-compare/fetch-ghostty.sh, then build again",
        ));
    }
    out
}

/// Run `visitor` once for every engine.
pub fn for_each_engine(visitor: &mut impl EngineVisitor) {
    visitor.visit::<AlacrittyEngine>();
    wormhole_vt_bench::engines::for_each_engine(visitor);
    visitor.visit::<WeztermEngine>();
    visitor.visit::<Vt100Engine>();
    #[cfg(has_ghostty)]
    visitor.visit::<GhosttyEngine>();
}

/// Run `visitor` for the engine called `name`; false when there is none.
pub fn visit_engine(name: &str, visitor: &mut impl EngineVisitor) -> bool {
    match name {
        AlacrittyEngine::NAME => visitor.visit::<AlacrittyEngine>(),
        WeztermEngine::NAME => visitor.visit::<WeztermEngine>(),
        Vt100Engine::NAME => visitor.visit::<Vt100Engine>(),
        #[cfg(has_ghostty)]
        GhosttyEngine::NAME => visitor.visit::<GhosttyEngine>(),
        other => return wormhole_vt_bench::engines::visit_engine(other, visitor),
    }
    true
}
