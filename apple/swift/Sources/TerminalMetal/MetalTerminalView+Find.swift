// Find's matches on screen, painted from the theme's find colours (`TerminalPalette.FindColors`). The search is the
// Rust side's (`TerminalScrollback.findSpans`); the view asks it for the matches of the viewport rows after each
// frame while the bar is open, and paints what it is told. The renderer only rebuilds rows whose marks changed.

import TerminalCore

extension MetalTerminalView {
    /// The matches among the viewport rows, the current one flagged; an empty list clears them.
    public func setFound(_ spans: [FindSpan]) {
        renderer.setFound(spans)
        requestRedraw()
    }

    /// After a frame, and when the bar opens or closes: the matches as the last frame shows them. Nothing is
    /// handed to the renderer while there are none and none are shown, so a closed bar costs a flag per frame.
    func refreshFound() {
        let spans = findActive ? (source.scrollback?.findSpans() ?? []) : []
        guard !spans.isEmpty || foundShown else { return }
        foundShown = !spans.isEmpty
        renderer.setFound(spans)
    }
}
