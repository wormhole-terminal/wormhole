// A pane's or divider's place in the tab, in points from its top left (`view/panes/geometry.rs`).

import Foundation

public struct PaneRect: Sendable, Equatable {
    public var x, y, w, h: Double

    public init(_ x: Double, _ y: Double, _ w: Double, _ h: Double) { (self.x, self.y, self.w, self.h) = (x, y, w, h) }

    public var right: Double { x + w }
    public var bottom: Double { y + h }
    public var center: (x: Double, y: Double) { (x + w / 2, y + h / 2) }

    /// The start and the extent along `axis`: x and w across a row, y and h down a column.
    func along(_ axis: Axis) -> (start: Double, extent: Double) { axis == .row ? (x, w) : (y, h) }
}

/// Which divider: the split it belongs to (child indexes from the root down) and the index of the child
/// before it. Stable while the tree's shape is, which a drag never changes.
public struct DividerID: Sendable, Equatable, Hashable {
    public var path: [Int]
    public var index: Int

    public init(path: [Int], index: Int) { (self.path, self.index) = (path, index) }
}

/// A line with no thickness along two children's shared edge; a row split's is vertical and drags in x.
public struct Divider: Sendable, Equatable {
    public var id: DividerID
    public var axis: Axis
    public var rect: PaneRect

    /// Where the line is along its split's axis.
    public var position: Double { rect.along(axis).start }
}

/// The least a pane may be, in points: 160 is about 18 columns at the default font and the narrowest the
/// agents' TUIs redraw without wrapping every line; 96 about five rows (`PANE_MIN_W`, `PANE_MIN_H`).
public enum PaneMin {
    public static let width = 160.0
    public static let height = 96.0
}
