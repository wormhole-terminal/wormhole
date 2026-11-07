// What the find bar needs from a terminal: the seam the Rust search (`wormhole_term::find`, which runs off the
// UI thread on a copy of the grid's text) will be put behind. Matches stay in Rust; Swift sees counts and positions.

import Foundation

public struct FindQuery: Equatable, Sendable {
    public var text: String
    public var options: FindOptions

    public init(text: String = "", options: FindOptions = FindOptions()) {
        (self.text, self.options) = (text, options)
    }
}

/// What one search found.
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

@MainActor
public protocol TerminalSearching: AnyObject {
    /// Searches the scrollback and the screen; answers when done. An empty query finds nothing.
    func search(_ query: FindQuery) async -> FindOutcome
    /// Scrolls match `index` of the last search into view.
    func reveal(match index: Int)
    /// Where the matches are along the scrollback, 0 at its top and 1 at the bottom of the screen.
    func ticks() -> [Double]
    /// The terminal's selected text, for ⌘E.
    func selectionText() -> String?
    /// The program printed since the last search (its output revision moved).
    var hasNewOutput: Bool { get }
}
