// A terminal's blocks held in memory, for previews, the gallery and tests: four commands on a 24-row screen,
// with the jumps and actions recorded instead of acted on.

import Foundation
import TerminalCore

@MainActor
public final class MockBlocks: CommandBlocks {
    public var blocks: [CommandBlock]
    public var displayOffset = 0
    public var shellAtPrompt = true
    public let rows: Int
    public private(set) var topLine = 0
    public private(set) var selected: ClosedRange<Int>?
    public private(set) var typed: [String] = []
    public var putOutcome = PutOutcome.typed

    public init(blocks: [CommandBlock] = MockBlocks.sample, rows: Int = 24) {
        (self.blocks, self.rows) = (blocks, rows)
    }

    public var bottommostLine: Int { rows - 1 }

    public var spans: [BlockSpan] {
        blocks.compactMap { block in
            let first = max(block.start - topLine, 0), last = min(block.end - topLine, rows - 1)
            guard first <= last else { return nil }
            return BlockSpan(id: block.id, rows: first..<(last + 1), starts: block.start >= topLine, state: block.state)
        }
    }

    public func block(id: UInt64) -> CommandBlock? { blocks.first { $0.id == id } }
    public func block(atLine line: Int) -> CommandBlock? { blocks.first { $0.start <= line && line <= $0.end } }

    public func newestRan() -> CommandBlock? { blocks.last { $0.state.ran } }

    public func promptLine(from line: Int, direction: PromptDirection) -> Int? {
        let starts = blocks.map(\.start)
        return direction == .up ? starts.last { $0 < line } : starts.first { $0 > line }
    }

    public func scrollLineToTop(_ line: Int) { topLine = line; displayOffset = max(bottommostLine - line - rows + 1, 1) }
    public func scrollToBottom() { topLine = 0; displayOffset = 0 }
    public func selectLines(_ lines: ClosedRange<Int>) { selected = lines }
    public func linesText(_ lines: ClosedRange<Int>) -> String { lines.map { "output line \($0)" }.joined(separator: "\n") }
    public func putAtPrompt(_ block: CommandBlock) -> PutOutcome {
        if putOutcome == .typed { typed.append(block.command) }
        return putOutcome
    }

    public static let sample: [CommandBlock] = [
        CommandBlock(id: 1, start: 0, outputStart: 1, end: 5, command: "git status",
                     state: .finished(code: 0, took: .milliseconds(380))),
        CommandBlock(id: 2, start: 6, outputStart: 7, end: 13, command: "cargo build --release",
                     state: .finished(code: 101, took: .seconds(42))),
        CommandBlock(id: 3, start: 14, outputStart: 15, end: 18, command: "ls",
                     state: .finished(code: nil, took: .milliseconds(30))),
        CommandBlock(id: 4, start: 19, end: 19, command: "", state: .prompt),
    ]
}
