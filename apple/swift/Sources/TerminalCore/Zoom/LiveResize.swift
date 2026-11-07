// A window dragged to another size (`view::motion::live_resize`): what the program in its terminal is told meanwhile,
// the window's resize steps and the strip a window off the cell grid leaves when the drag ends. A full-screen program
// redraws its whole screen for every size it is told, so during a live resize the PTY is resized at most every 100 ms,
// the last size always getting through, and once more the moment the drag ends. Pure timing and geometry: asking AppKit
// whether a drag is on (`NSWindow.inLiveResize`) and hearing it end are the window's.

import CoreGraphics
import Foundation

public struct ResizeGate: Sendable {
    public struct Step: Equatable, Sendable {
        /// Tell the program the size the window gives it now.
        public var apply: Bool
        /// Draw this terminal again after this long: a size is held back and this is when it is due.
        public var wakeIn: TimeInterval?
    }

    /// The least time between two sizes told to the program during a live resize.
    public static let interval: TimeInterval = 0.1
    private var applied: Date?

    public init() {}

    /// The frame at `now`: `live` says the window is being dragged, `differs` that the size the window gives the
    /// terminal is not the one the program has. The caller does what `apply` says; the gate counts it as done.
    public mutating func step(now: Date, live: Bool, differs: Bool) -> Step {
        let since = applied.map { now.timeIntervalSince($0) }
        let due = since.map { $0 >= Self.interval } ?? true
        let apply = differs && (!live || due)
        // Only what is told during a live resize counts: a size told just before the person took hold of the edge
        // does not make the first one of the drag wait.
        if apply { applied = live ? now : nil }
        let wake: TimeInterval? = if let since, live, differs, !apply { max(Self.interval - since, 0) } else { nil }
        return Step(apply: apply, wakeIn: wake)
    }
}

public enum ResizeSteps {
    /// A leftover strip narrower than this is the rounding of a fractional cell to the screen's pixels.
    static let settleMin: CGFloat = 1
    /// How near a whole point a cell's width must be for the window to step by it: `NSWindow.resizeIncrements` keeps
    /// whole points, so any other width would be followed wrongly on most steps.
    static let wholePoint: CGFloat = 0.05

    /// The steps the window resizes in for a terminal whose cell is `cell`; a width that is not whole is left free
    /// (one point) and `settle` takes the strip off at the end.
    public static func increments(cell: CGSize) -> CGSize {
        let width = abs(cell.width - cell.width.rounded()) <= wholePoint ? max(cell.width.rounded(), 1) : 1
        return CGSize(width: width, height: max(cell.height.rounded(), 1))
    }

    /// How far the window is to shrink, across and down, so it holds whole cells with no strip left: `spare` is what
    /// the terminal's area has beyond its grid. Nil when nothing worth moving for is left, and never a whole cell.
    public static func settle(spare: CGSize, cell: CGSize) -> CGSize? {
        func axis(_ spare: CGFloat, _ cell: CGFloat) -> CGFloat { spare >= settleMin && spare < cell ? spare : 0 }
        let (dx, dy) = (axis(spare.width, cell.width), axis(spare.height, cell.height))
        return dx > 0 || dy > 0 ? CGSize(width: dx, height: dy) : nil
    }
}

extension ResizeSteps {
    /// How near a screen's edges a window's sides are for it to count as tiled or snapped (`TILE_MARGIN`).
    static let tileMargin: CGFloat = 24

    /// The frame (AppKit's space: `origin.y` is the bottom edge) the window is to take off `strip`, or nil when the window
    /// is the system's size, not the person's: tiled or filling the screen's width or height. The edges the person did not
    /// hold stay where they are (`settled_frame`, `tiled`). `heldLeft` and `heldTop` are the edges the drag moved.
    public static func settledFrame(_ frame: CGRect, strip: CGSize, visible: CGRect, heldLeft: Bool, heldTop: Bool) -> CGRect? {
        let across = frame.minX <= visible.minX + tileMargin && frame.maxX >= visible.maxX - tileMargin
        let down = frame.minY <= visible.minY + tileMargin && frame.maxY >= visible.maxY - tileMargin
        guard !across, !down else { return nil }
        return CGRect(
            x: heldLeft ? frame.minX + strip.width : frame.minX, y: heldTop ? frame.minY : frame.minY + strip.height,
            width: frame.width - strip.width, height: frame.height - strip.height)
    }
}
