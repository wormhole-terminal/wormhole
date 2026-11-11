// The selected terminal's split as the core lays it out for the pane area (core `PanesSnapshot`): where each pane is, which
// has the keyboard, which is zoomed and the lines between them. The tree, the minimum sizes and every rule are the core's;
// the window draws these and sends `WorkspaceAction`s (a drop, a divider, a focus).

import Foundation

/// A pane's id: the id of its terminal.
public typealias PaneID = Int

/// How a split lays its children out: `row` side by side (⌘D adds to it), `column` stacked (⇧⌘D).
public enum SplitAxis: Sendable, Equatable, Hashable { case row, column }

/// Where ⌥⌘← → ↑ ↓ moves the focus.
public enum Direction: Sendable, Equatable, Hashable, CaseIterable { case left, right, up, down }

/// The side of a pane a drop lands on, which is the side the dropped pane ends up on.
public enum Zone: Sendable, CaseIterable, Equatable, Hashable { case left, right, top, bottom }

/// A pane's or divider's place in the area, in points from its top left.
public struct PaneRect: Sendable, Equatable {
    public var x, y, w, h: Double

    public init(_ x: Double, _ y: Double, _ w: Double, _ h: Double) { (self.x, self.y, self.w, self.h) = (x, y, w, h) }

    public var right: Double { x + w }
    public var bottom: Double { y + h }
}

/// A line between two panes: the split it belongs to (child indexes from the root down) and the index of the child before it
/// name it to the core; `position` is where it lies along its axis (x in a row's, y in a column's).
public struct PaneDividerState: Sendable, Equatable {
    public var path: [UInt32]
    public var index: UInt32
    public var axis: SplitAxis
    public var position: Double
    public var rect: PaneRect

    public init(path: [UInt32], index: UInt32, axis: SplitAxis, position: Double, rect: PaneRect) {
        (self.path, self.index, self.axis, self.position, self.rect) = (path, index, axis, position, rect)
    }

    /// What the core calls the divider by.
    public struct ID: Sendable, Equatable, Hashable {
        public var path: [UInt32]
        public var index: UInt32
    }

    public var id: ID { ID(path: path, index: index) }
}

public struct PanesState: Sendable, Equatable {
    public var shown: [PanePlace]
    /// Every pane at its place unzoomed, in reading order: what a zoom hides, and the order the headers count in.
    public var unzoomed: [PanePlace]
    public var focused: PaneID?
    public var zoomed: PaneID?
    public var dividers: [PaneDividerState]

    public init(
        shown: [PanePlace] = [], unzoomed: [PanePlace]? = nil, focused: PaneID? = nil, zoomed: PaneID? = nil,
        dividers: [PaneDividerState] = []
    ) {
        (self.shown, self.unzoomed, self.focused, self.zoomed, self.dividers) = (shown, unzoomed ?? shown, focused, zoomed, dividers)
    }

    /// The panes in reading order.
    public var order: [PaneID] { unzoomed.map(\.id) }
    public var isSplit: Bool { unzoomed.count > 1 }
}

/// A pane and its place.
public struct PanePlace: Sendable, Equatable {
    public var id: PaneID
    public var rect: PaneRect

    public init(_ id: PaneID, _ rect: PaneRect) { (self.id, self.rect) = (id, rect) }
}
