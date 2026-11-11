// What one search of a terminal's scrollback found (`wormhole_term::find`); matches stay in Rust, Swift sees counts.

import Foundation

public struct FindOutcome: Equatable, Sendable {
    public var count: Int
    /// The search stopped at its cap (`MAX_MATCHES`, 10,000): the count reads "10,000+".
    public var capped: Bool
    /// A regular expression that does not parse.
    public var invalid: Bool
    /// The match nearest the bottom of the view, where a fresh search starts.
    public var nearest: Int?

    public init(count: Int = 0, capped: Bool = false, invalid: Bool = false, nearest: Int? = nil) {
        (self.count, self.capped, self.invalid, self.nearest) = (count, capped, invalid, nearest)
    }
}
