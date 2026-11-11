//! What a Probe's rules and hooks add to the rows: tones on matched text, text shortened or redacted
//! on screen, and the tint of a block a hook marked (`wormhole_core::probe::runtime::Decor` works the spans out
//! per logical line, bounded by the row cache's own damage). Display only: the grid, the selection, find, copy
//! and the saved text never see any of it.

use gpui_kit::App;
use wormhole_core::probe::runtime::Inputs;
use wormhole_term::ScreenSnapshot;

use crate::terminal_view::TerminalView;

pub(super) use wormhole_core::probe::runtime::{Decor, RuleSpan};

impl TerminalView {
    /// What Probes add to the rows of `snapshot`, worked out into `decor` before the rows are synced: the rules
    /// in force, the span the pointer reveals and the blocks hooks marked. With nothing in force it clears it.
    pub(super) fn probe_marks(&self, decor: &mut Decor, snapshot: &ScreenSnapshot, cx: &App) {
        let rules = crate::probe_runtime::rules(cx);
        let tab = self.tab;
        let block_tone = |block: u64| crate::probe_runtime::block_tone(tab, block, cx);
        decor.update(
            snapshot,
            &Inputs {
                rules: rules.as_deref(),
                generation: crate::probe_runtime::generation(cx),
                reveal: self.reveal.at,
                block_tone: &block_tone,
            },
        );
    }
}
