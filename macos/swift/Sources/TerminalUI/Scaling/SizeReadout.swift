// The chip with the grid's size in the middle of the terminal (`view::motion::size_readout`, `scaling/readout.rs`):
// whole for 700 ms after the last change of size, then fading over 150 ms; Reduce Motion cuts it at 700 ms. It says
// the size when a live resize changed the grid or a text zoom has settled; the first size a terminal is seen at says
// nothing, never on open, and a zoom older than 2 s when the grid settles says nothing (the terminal was off screen).

import Foundation
import Observation

public enum SizeReadoutPhase: Equatable, Sendable {
    /// Whole, for this much longer.
    case holding(TimeInterval)
    /// Fading: this much of it still shows, 1 down to 0.
    case fading(Double)
    case gone

    public static let hold: TimeInterval = 0.7
    public static let fade: TimeInterval = 0.15

    /// The phase `since` the last change; `cut` (Reduce Motion) goes from whole to gone without fading.
    public static func of(since: TimeInterval, cut: Bool) -> SizeReadoutPhase {
        if since < hold { return .holding(hold - since) }
        let into = since - hold
        if cut || into >= fade { return .gone }
        return .fading(1 - into / fade)
    }
}

@MainActor @Observable
public final class SizeReadoutModel {
    public private(set) var text: String?
    public private(set) var since = Date.distantPast

    @ObservationIgnored private var wanted: (cols: Int, rows: Int)?
    @ObservationIgnored private var zoomDue: Date?
    /// A text zoom older than this when a frame settles the grid says nothing.
    static let dueFor: TimeInterval = 2

    public init() {}

    /// The text was zoomed: the chip says the size the grid has once it is settled. A terminal not on screen says
    /// nothing, or it would pop a chip when switched to.
    public func textZoomed(onScreen: Bool, at now: Date = Date()) { zoomDue = onScreen ? now : nil }

    /// A frame found the pane making a grid of `cols` × `rows`.
    public func noteSize(cols: Int, rows: Int, live: Bool, at now: Date = Date()) {
        let changed = wanted.map { $0.cols != cols || $0.rows != rows } ?? false
        wanted = (cols, rows)
        let zoomed = zoomDue.map { now.timeIntervalSince($0) < Self.dueFor } ?? false
        zoomDue = nil
        if (live && changed) || zoomed {
            text = L10n.sizeReadout(cols, rows)
            since = now
        }
    }
}
