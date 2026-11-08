// A terminal's folds as the window shows them: the marker rows over the grid, a click on one, and Edit ▸ Show Hidden Lines.
// What may be folded and when a fold shows is Rust's (`wormhole_term::folds`); this reads the frame's markers and relays gestures.

import AppKit
import TerminalCore
import TerminalMetal

extension TerminalOverlay {
    /// Read the markers of the rows just drawn; the observed properties change only when what they say does.
    func syncFolds() {
        guard let view else { return }
        let now = view.foldMarkers
        if now != markers { markers = now }
        let folds = view.source.probes?.hasFolds ?? false
        if folds != hasFolds { hasFolds = folds }
    }

    /// The marker was clicked, or VoiceOver's Show: opening says how many lines it showed.
    func toggle(_ marker: FoldMarker) {
        guard let view, view.toggleFold(marker.id), !marker.isOpen, let words = view.source.probes?.foldShownWords(lines: marker.hidden) else { return }
        Self.announce(words)
    }

    /// Edit ▸ Show Hidden Lines.
    func showHiddenLines() {
        guard let view else { return }
        let shown = view.showHiddenLines()
        if shown > 0, let words = view.source.probes?.foldShownWords(lines: shown) { Self.announce(words) }
    }
}
