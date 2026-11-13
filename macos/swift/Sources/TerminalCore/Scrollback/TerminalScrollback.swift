// What a terminal's scrollback gives the views around the grid, over the real engine: find through the whole
// history, the command blocks and the minimap's lines. Three small protocols, so each user names only what it
// calls; the source hands out the whole (`FrameSource.scrollback`). Every call is one seam call; the ones that
// search or read a copy of the grid say so and belong off the main thread.

import Foundation

public protocol TerminalFinding: AnyObject, Sendable {
    /// Searches the history and the screen (`wormhole_term::find`): copies the text under the engine's lock and
    /// searches the copy, so call it off the main thread. An empty query finds nothing.
    func find(_ text: String, matchCase: Bool, wholeWord: Bool, regex: Bool) -> FindOutcome
    /// Makes match `index` of the last search current and scrolls it into view; the next frame is complete.
    func findReveal(match index: Int)
    /// The matches on the viewport rows of the last frame, the current one flagged.
    func findSpans() -> [FindSpan]
    /// The fold markers of the last frame as they read while a search is open (how many matches are inside each).
    func findFoldWords() -> [FoldWords]
    /// Where the matches are along the scrollback, 0 at its top and 1 at the bottom of the screen.
    func findTicks() -> [Double]
    /// The program printed since the last search: its results may be stale.
    var findHasNewOutput: Bool { get }
    /// Forgets the results (the bar closed).
    func findClear()
}

public protocol TerminalBlocks: AnyObject, Sendable {
    /// The shell has integration: a prompt mark has arrived.
    var blocksAvailable: Bool { get }
    /// The shell is at its prompt, which is when a command can be typed there.
    var shellAtPrompt: Bool { get }
    /// The blocks in the viewport of the last frame, top to bottom.
    func blockSpans() -> [BlockSpan]
    func block(id: UInt64) -> CommandBlock?
    func block(atLine line: Int) -> CommandBlock?
    /// The newest block whose command ran.
    func newestRanBlock() -> CommandBlock?
    /// The previous or next prompt from `line`, strictly.
    func promptLine(from line: Int, direction: PromptDirection) -> Int?
    var displayOffset: Int { get }
    var bottommostLine: Int { get }
    /// The grid line the view's top row shows (folds make it differ from `-displayOffset`).
    var topLine: Int { get }
    func scrollLineToTop(_ line: Int)
    func scrollToBottom()
    func selectLines(_ lines: ClosedRange<Int>)
    func linesText(_ lines: ClosedRange<Int>) -> String
    /// Types the block's command at the prompt after clearing the line, never running it, if all of it shows as
    /// typed. `palette` is the theme's colours by slot (0xRRGGBB), to tell two names for one colour from two colours.
    func putAtPrompt(id: UInt64, palette: [UInt32]) -> PutOutcome
}

public typealias TerminalScrollback = TerminalFinding & TerminalBlocks & TerminalMinimap
