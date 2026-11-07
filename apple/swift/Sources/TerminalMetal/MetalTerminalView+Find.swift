// Find's matches on screen, painted from the theme's find colours (`TerminalPalette.FindColors`). The search is the
// Rust side's; the host asks it for the matches of the viewport rows (after each frame while the bar is open)
// and hands them here.

import TerminalCore

extension MetalTerminalView {
    /// The matches among the viewport rows, the current one flagged; an empty list clears them.
    public func setFound(_ spans: [FindSpan]) {
        renderer.setFound(spans)
        requestRedraw()
    }
}
