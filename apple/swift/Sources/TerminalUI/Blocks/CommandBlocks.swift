// What command blocks need from a terminal: OSC 133 marks the shell sends, read from the grid by
// `wormhole-term` (`blocks.rs`, `marks.rs`). The seam the FFI will back; lines are grid lines (0 the top of the
// live screen, history above it negative), rows are viewport rows.

import Foundation

/// Where a command stands, from the shell's own marks.
public enum BlockState: Equatable, Sendable {
    case prompt
    case running
    case finished(code: Int?, took: Duration?)

    /// A command ran here: the shell reported it started or gave an exit code (`BlockState::ran`).
    public var ran: Bool {
        switch self {
        case .prompt: false
        case .running: true
        case let .finished(code, took): code != nil || took != nil
        }
    }
}

/// A block as it shows in the viewport: the rows it covers, for the gutter mark and the hover tint.
public struct BlockSpan: Equatable, Sendable, Identifiable {
    public let id: UInt64
    public var rows: Range<Int>
    /// `rows.lowerBound` is the block's first row; otherwise it began above the viewport.
    public var starts: Bool
    public var state: BlockState

    public init(id: UInt64, rows: Range<Int>, starts: Bool = true, state: BlockState) {
        (self.id, self.rows, self.starts, self.state) = (id, rows, starts, state)
    }
}

/// A block on the grid, with its command.
public struct CommandBlock: Equatable, Sendable, Identifiable {
    public let id: UInt64
    public var start: Int
    public var outputStart: Int?
    public var end: Int
    public var command: String
    public var state: BlockState

    public init(id: UInt64, start: Int, outputStart: Int? = nil, end: Int, command: String, state: BlockState) {
        (self.id, self.start, self.outputStart, self.end, self.command, self.state) =
            (id, start, outputStart, end, command, state)
    }

    /// The output's lines, if there is any.
    public var outputLines: ClosedRange<Int>? { outputStart.flatMap { $0 <= end ? $0...end : nil } }
}

public enum PromptDirection: Sendable { case up, down }

/// Why the engine would not type a command at the prompt (`Refusal`).
public enum PutRefusal: Sendable, Equatable { case concealed, sameColor, invisible, lines }

public enum PutOutcome: Sendable, Equatable {
    case typed
    /// A program holds the terminal: it would get the text as input.
    case notAtPrompt
    case refused(PutRefusal)
}

@MainActor
public protocol CommandBlocks: AnyObject {
    /// The viewport's blocks, as the last frame drew them.
    var spans: [BlockSpan] { get }
    var displayOffset: Int { get }
    var bottommostLine: Int { get }
    func block(id: UInt64) -> CommandBlock?
    func block(atLine line: Int) -> CommandBlock?
    /// The previous or next prompt from `line`, strictly.
    func promptLine(from line: Int, direction: PromptDirection) -> Int?
    func scrollLineToTop(_ line: Int)
    func scrollToBottom()
    func selectLines(_ lines: ClosedRange<Int>)
    func linesText(_ lines: ClosedRange<Int>) -> String
    /// Types the block's command at the prompt for the person to run, if it all shows as typed.
    func putAtPrompt(_ block: CommandBlock) -> PutOutcome
}
