// What the find bar needs from a terminal: the seam the Rust search (`wormhole_term::find`, which runs off the
// UI thread on a copy of the grid's text) will be put behind. Matches stay in Rust; Swift sees counts and positions.

import Foundation
import TerminalCore

public struct FindQuery: Equatable, Sendable {
    public var text: String
    public var options: FindOptions

    public init(text: String = "", options: FindOptions = FindOptions()) {
        (self.text, self.options) = (text, options)
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
    /// The bar opened or closed: matches are painted in the terminal only while it is open.
    func setActive(_ active: Bool)
}
