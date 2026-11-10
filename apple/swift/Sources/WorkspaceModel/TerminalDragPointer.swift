// A terminal dragged out of the sidebar's list over the panes. The list's drag is its own pointer gesture (so the
// lifted row, the mark and the settle are ours), which a drop target elsewhere in the window cannot see: while the
// pointer is outside the list the list reports where it is here, in window coordinates, and the pane area (which owns
// the zones and the rules of a join) reads it, tints the side a drop would take and takes the drop. Plain values and
// no UI framework, like the rest of this module; the window creates one and hands it to both sides.

import CoreGraphics
import Observation

@MainActor @Observable
public final class TerminalDragPointer {
    /// A terminal and where the pointer is, in the window's coordinates (origin top left).
    public struct Over: Equatable, Sendable {
        public var terminal: TerminalID
        public var point: CGPoint

        public init(terminal: TerminalID, point: CGPoint) { (self.terminal, self.point) = (terminal, point) }
    }

    /// The pointer is outside the list, dragging this terminal.
    public private(set) var over: Over?
    /// The button went up outside the list: the one who takes the drop acts and calls `settle()`.
    public private(set) var released: Over?

    /// Asked first when a terminal is let go, with the pointer in the window's coordinates (it may lie outside the window):
    /// true when the window took it, by opening a window of its own for it or giving it to another one.
    public var tearOff: (@MainActor (TerminalID, CGPoint) -> Bool)?

    public init() {}

    public func move(_ terminal: TerminalID, to point: CGPoint) { over = Over(terminal: terminal, point: point) }

    /// The pointer came back over the list, or the drag was cancelled.
    public func leave() { if over != nil { over = nil } }

    public func release(_ terminal: TerminalID, at point: CGPoint) {
        over = nil
        released = Over(terminal: terminal, point: point)
    }

    /// The release was answered.
    public func settle() { released = nil }
}
