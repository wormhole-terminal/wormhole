// The view's side of the scroll bar: feeding it each frame's history and offset, and turning its hits into scrolling.

import AppKit

extension MetalTerminalView {
    func updateScroller() {
        guard let header = renderer.scene.header else { return }
        scroller.update(history: header.historySize, rows: header.rows, offset: header.displayOffset)
    }

    /// The scroll bar asked for the view to be `offset` lines back from the live screen.
    func scroll(toOffset offset: Int) {
        let current = renderer.scene.header?.displayOffset ?? 0
        guard offset != current else { return }
        source.scroll(lines: offset - current)
        requestRedraw()
    }

    func revealScrollerNear(_ point: CGPoint) { scroller.pointer(x: point.x, in: bounds.width) }
}
