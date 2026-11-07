// The minimap's side of the scrollback: where the view is among all the lines, and the lines as runs of one colour.
// Rust reads them only when the output or the view changed; `stamp` moves when `minimapLines` would differ.

import Foundation

/// Columns `start..<end` of a line drawn as one block, in the colours of `cell` (its first cell): the text colour
/// for text, the background for coloured blanks.
public struct MinimapRun: Equatable, Sendable {
    public var start: Int
    public var end: Int
    public var isText: Bool
    public var cell: PackedCell

    public init(start: Int, end: Int, isText: Bool, cell: PackedCell) {
        (self.start, self.end, self.isText, self.cell) = (start, end, isText, cell)
    }
}

/// How many lines there are and which the view shows (`wormhole_term::minimap::MinimapGeometry`).
public struct MinimapRead: Equatable, Sendable {
    public var total: Int
    public var screen: Int
    public var fits: Int
    public var viewTop: Int
    public var columns: Int
    public var stamp: UInt64

    public init(total: Int, screen: Int, fits: Int, viewTop: Int, columns: Int, stamp: UInt64) {
        (self.total, self.screen, self.fits, self.viewTop, self.columns, self.stamp) =
            (total, screen, fits, viewTop, columns, stamp)
    }
}

public protocol TerminalMinimap: AnyObject, Sendable {
    /// Brings the copy up to date for a minimap with room for `fits` lines.
    func minimapRead(fits: Int) -> MinimapRead
    /// The runs of each line shown, from the geometry's first line on.
    func minimapLines() -> [[MinimapRun]]
    /// Scrolls the view so it is `offset` lines into the history (0 the live screen).
    func scroll(toDisplayOffset offset: Int)
}
