// What the pane's command-block views need from a terminal: the spans of the last frame, the blocks on the grid and
// the actions on them. The value types are TerminalCore's (`BlockTypes.swift`); `CoreBlocks` backs the protocol with
// the real terminal, `MockBlocks` with a list in memory.

import Foundation
import TerminalCore

@MainActor
public protocol CommandBlocks: AnyObject {
    /// The viewport's blocks, as the last frame drew them.
    var spans: [BlockSpan] { get }
    var displayOffset: Int { get }
    /// The shell is at its prompt, which is when a command can be typed there.
    var shellAtPrompt: Bool { get }
    var bottommostLine: Int { get }
    var topLine: Int { get }
    func block(id: UInt64) -> CommandBlock?
    func block(atLine line: Int) -> CommandBlock?
    /// The newest block whose command ran, anywhere on the grid.
    func newestRan() -> CommandBlock?
    /// The previous or next prompt from `line`, strictly.
    func promptLine(from line: Int, direction: PromptDirection) -> Int?
    func scrollLineToTop(_ line: Int)
    func scrollToBottom()
    func selectLines(_ lines: ClosedRange<Int>)
    func linesText(_ lines: ClosedRange<Int>) -> String
    /// Types the block's command at the prompt for the person to run, if it all shows as typed.
    func putAtPrompt(_ block: CommandBlock) -> PutOutcome
}
