// Probes on the terminal: the marker rows of folds, a shortened span showing its text under the pointer, and the file a
// rule's link names. The decisions are Rust's (`TerminalProbes`); the view reports the pointer and hands the host
// what it must draw or open.

import TerminalCore

extension MetalTerminalView {
    /// The fold markers among the viewport rows, as the last frame carried them: the host draws them over the blank rows
    /// the grid leaves for them (a real, focusable, accessible element each).
    public var foldMarkers: [FoldMarkerRow] {
        renderer.scene.markers.enumerated().compactMap { row, marker in marker.map { FoldMarkerRow(row: row, marker: $0) } }
    }

    /// The pointer is over `hit` (nil: it left), with Option as `alt`: a span a rule shortened or redacted shows its text
    /// while the pointer is over it, when its rule lets it. Nothing is asked while no rule marks anything.
    func updateReveal(under hit: CellHit?, alt: Bool) {
        guard let probes = source.probes, renderer.scene.hasRuleSpans || revealing else { return }
        // The span that shows changed: Rust sends every row of the next frame, so a frame is wanted.
        guard probes.setReveal(row: hit?.row, column: hit?.column, alt: alt) else { return }
        revealing = hit != nil
        requestRedraw()
    }

    /// The marker was clicked (or VoiceOver's Show): open or close fold `id`; the lines it showed, when it opened.
    @discardableResult
    public func toggleFold(_ id: UInt64) -> Bool {
        guard let probes = source.probes, probes.foldToggle(id) else { return false }
        requestRedraw()
        return true
    }

    /// Edit ▸ Show Hidden Lines: every fold open; how many lines that showed.
    public func showHiddenLines() -> Int {
        guard let probes = source.probes else { return 0 }
        let shown = probes.foldShowAll()
        if shown > 0 { requestRedraw() }
        return shown
    }
}
