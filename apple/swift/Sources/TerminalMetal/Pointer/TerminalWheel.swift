// The wheel and the trackpad. A wheel reports lines, a trackpad points (with momentum, which AppKit keeps
// sending as ordinary events); both become cells here and the seam adds them up and decides what they mean:
// wheel buttons for a program that asked for the mouse, arrow keys on the alternate screen, else the history.

import AppKit
import TerminalCore

extension MetalTerminalView {
    public override func scrollWheel(with event: NSEvent) {
        guard let interaction else { return }
        let grid = cellGrid
        let precise = event.hasPreciseScrollingDeltas
        let lines = precise ? event.scrollingDeltaY / grid.cellHeight : event.scrollingDeltaY
        let columns = precise ? event.scrollingDeltaX / grid.cellWidth : event.scrollingDeltaX
        let point = convert(event.locationInWindow, from: nil)
        let hit = grid.hit(point) ?? CellHit(row: 0, column: 0, side: .left, x: 0, y: 0)
        let flags = event.modifierFlags
        interaction.scrollWheel(
            WheelInput(
                lines: Float(lines), columns: Float(columns), row: hit.row, column: hit.column, x: hit.x, y: hit.y,
                modifiers: PointerModifiers(shift: flags.contains(.shift), alt: flags.contains(.option), control: flags.contains(.control))))
        // The link under the pointer moved with the text.
        updateLink(under: nil, command: false)
        requestRedraw()
    }
}
