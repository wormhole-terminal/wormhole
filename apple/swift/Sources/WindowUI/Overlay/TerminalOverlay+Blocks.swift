// The pane's command blocks, as the overlay hands them to the window: the hit rows for the hover, the grid's geometry
// for the marks, and VoiceOver's word for a jump.

import AppKit
import TerminalUI

extension TerminalOverlay {
    /// Where the rows are in the pane, for the marks and the hover; nil before the view has a grid.
    var blockGeometry: BlockGeometry? {
        guard let renderer = view?.renderer, renderer.gridSize.rows > 0 else { return nil }
        let cell = renderer.cellPixels
        return BlockGeometry(rows: renderer.gridSize.rows, cellHeight: CGFloat(cell.height) / CGFloat(renderer.scale), gutter: 0)
    }

    /// The terminal has a real scrollback (blocks and the minimap exist only over the engine).
    var hasScrollback: Bool { view?.source.scrollback != nil }

    /// The view changed by what an action did (a selection, a scroll): draw it.
    func redraw() { view?.requestRedraw() }

    /// An RGB slot as the engine reads it (`0xRRGGBB`).
    static func rgb(_ colour: SIMD3<UInt8>) -> UInt32 { UInt32(colour.x) << 16 | UInt32(colour.y) << 8 | UInt32(colour.z) }

    static func announce(_ text: String) {
        NSAccessibility.post(element: NSApp as Any, notification: .announcementRequested, userInfo: [.announcement: text])
    }
}
