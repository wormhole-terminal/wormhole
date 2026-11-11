// A drag on a column's edge, as the GPUI app resolves it (`view::layout::resize`): the width follows the
// pointer between the column's minimum and maximum, sticks at the minimum for a little more, then lets the
// column close; dragging back out opens it again in the same gesture. The readout says why it stopped.

import CoreGraphics
import WordsModel

struct ColumnResize: Equatable {
    /// Dragging this far past the minimum closes the column.
    static let collapseSlack: CGFloat = 70

    enum Stop { case min, max }

    var open: Bool
    var width: CGFloat
    var stop: Stop?

    /// `extent` is the pointer's distance from the window edge the column hangs on.
    init(extent: CGFloat, range: ClosedRange<CGFloat>, max limit: CGFloat) {
        let high = Swift.max(limit, range.lowerBound)
        open = extent >= range.lowerBound - Self.collapseSlack
        width = Swift.min(Swift.max(extent, range.lowerBound), high)
        stop = !open ? nil : extent < range.lowerBound ? .min : extent > high ? .max : nil
    }

    /// The words beside the pointer: the width and why it stopped, or what letting go here does.
    func readout(closing: String) -> String {
        guard open else { return closing }
        let number = Int(width.rounded())
        switch stop {
        case nil: return "\(number)"
        case .min: return Words.text("window-resize-min", ["width": .whole(number)], fallback: "\(number) · min")
        case .max: return Words.text("window-resize-max", ["width": .whole(number)], fallback: "\(number) · max")
        }
    }
}
