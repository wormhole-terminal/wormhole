// A find match on screen: its columns on one viewport row, and whether it is the one Return would go to next.
// The search itself is the Rust side's (`wormhole-term` `find`); the renderer only paints what it is told.

import Foundation

public struct FindSpan: Sendable, Equatable {
    public var row: Int
    public var first: Int
    public var last: Int
    public var isCurrent: Bool

    public init(row: Int, first: Int, last: Int, isCurrent: Bool = false) {
        self.row = row
        self.first = first
        self.last = last
        self.isCurrent = isCurrent
    }
}
