// What the minimap needs from a terminal: its lines as runs of one colour, read by `wormhole-term` under the
// terminal's lock and only when the output or the view changed (`MinimapSnapshot::update`), and a way to scroll.

import Foundation
import TerminalCore

public struct MinimapSnapshot: Equatable, Sendable {
    public var geometry: MinimapGeometry
    public var columns: Int
    /// The runs of each line shown, from `geometry.first` on.
    public var lines: [[MinimapRun]]
    /// The collapsed output folds as lines from the oldest one: a tick at the left edge, as long as the lines the view folds into one marker.
    public var folds: [ClosedRange<Int>]

    public init(geometry: MinimapGeometry, columns: Int, lines: [[MinimapRun]], folds: [ClosedRange<Int>] = []) {
        (self.geometry, self.columns, self.lines, self.folds) = (geometry, columns, lines, folds)
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
