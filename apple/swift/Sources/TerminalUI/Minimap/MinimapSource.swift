// What the minimap needs from a terminal: its lines as runs of one colour, read by `wormhole-term` under the
// terminal's lock and only when the output or the view changed (`MinimapSnapshot::update`), and a way to scroll.

import Foundation
import TerminalCore

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

public struct MinimapSnapshot: Equatable, Sendable {
    public var geometry: MinimapGeometry
    public var columns: Int
    /// The runs of each line shown, from `geometry.first` on.
    public var lines: [[MinimapRun]]

    public init(geometry: MinimapGeometry, columns: Int, lines: [[MinimapRun]]) {
        (self.geometry, self.columns, self.lines) = (geometry, columns, lines)
    }
}

@MainActor
public protocol MinimapSource: AnyObject {
    /// The snapshot for a minimap with room for `fits` lines.
    func snapshot(fits: Int) -> MinimapSnapshot
    /// Scrolls the view so it is `offset` lines into the history.
    func scroll(toDisplayOffset offset: Int)
    /// The palette the runs' colours are resolved against.
    var palette: TerminalPalette { get }
}
