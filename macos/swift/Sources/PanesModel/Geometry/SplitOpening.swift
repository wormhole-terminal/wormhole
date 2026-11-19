// A split opening (⌘D, ⇧⌘D) as the window plays it: one pane gave half of its room to the new one, whose edge springs in
// from the far end of the pane that was split. Whether a change of layout is one, and where the edge is at each moment, are
// the core's (`view::motion::split`); the window keeps every terminal at one size while it moves and tells the layout's once,
// when it ends.

import Foundation
import WorkspaceModel

public struct SplitOpening: Sendable, Equatable {
    /// The pane that was split.
    public var target: PaneID
    /// The new pane is to the right of it (a row); else below it.
    public var row: Bool
    /// The room the pane that was split had, and the room the new pane ends in.
    public var whole, newPane: PaneRect
    /// How long it takes, in seconds: until the spring is at rest.
    public var duration: Double

    public init(target: PaneID, row: Bool, whole: PaneRect, newPane: PaneRect, duration: Double) {
        (self.target, self.row, self.whole, self.newPane, self.duration) = (target, row, whole, newPane, duration)
    }
}

/// The two rooms a moment into the opening.
public struct SplitFrame: Sendable, Equatable {
    /// What is left of the room of the pane that was split: it is clipped to this and never sized to it.
    public var kept: PaneRect
    /// The new pane's room at its own size, its leading edge where the spring has it.
    public var newPane: PaneRect

    public init(kept: PaneRect, newPane: PaneRect) { (self.kept, self.newPane) = (kept, newPane) }
}

/// The core's rule for it, behind a seam so the views link no bridge.
public protocol SplitMotion: Sendable {
    /// The opening that takes the panes `before` to `after`, `new` being the one that came; nil when it is anything else
    /// (a cut). Neither layout has a pane zoomed.
    func opening(before: [PanePlace], after: [PanePlace], new: PaneID) -> SplitOpening?
    /// Where the rooms are `elapsed` seconds in.
    func frame(of opening: SplitOpening, elapsed: Double) -> SplitFrame
}
