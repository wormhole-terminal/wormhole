// The blocks half of `UniFFIScrollback`: records of the generated bindings become TerminalCore's block types.

import Foundation
import TerminalCore
import WormholeBindings

extension UniFFIScrollback {
    var blocksAvailable: Bool { terminal.blocksAvailable() }
    var shellAtPrompt: Bool { terminal.shellAtPrompt() }

    func blockSpans() -> [BlockSpan] {
        terminal.blockSpans().map {
            BlockSpan(id: $0.id, rows: Int($0.firstRow)..<Int($0.endRow), starts: $0.starts, state: Self.state($0.status))
        }
    }

    func block(id: UInt64) -> CommandBlock? { terminal.blockById(id: id).map(Self.block) }
    func block(atLine line: Int) -> CommandBlock? { terminal.blockAtLine(line: Int32(clamping: line)).map(Self.block) }
    func newestRanBlock() -> CommandBlock? { terminal.blockNewestRan().map(Self.block) }

    func promptLine(from line: Int, direction: PromptDirection) -> Int? {
        terminal.promptLine(from: Int32(clamping: line), up: direction == .up).map(Int.init)
    }

    var displayOffset: Int { Int(terminal.displayOffset()) }
    var bottommostLine: Int { Int(terminal.bottommostLine()) }
    func scrollLineToTop(_ line: Int) { terminal.scrollLineToTop(line: Int32(clamping: line)) }
    func scrollToBottom() { terminal.scrollToBottom() }

    func selectLines(_ lines: ClosedRange<Int>) {
        terminal.selectLines(first: Int32(clamping: lines.lowerBound), last: Int32(clamping: lines.upperBound))
    }

    func linesText(_ lines: ClosedRange<Int>) -> String {
        terminal.linesText(first: Int32(clamping: lines.lowerBound), last: Int32(clamping: lines.upperBound))
    }

    func putAtPrompt(id: UInt64, palette: [UInt32]) -> PutOutcome {
        switch terminal.blockPutAtPrompt(id: id, palette: palette) {
        case .typed: .typed
        case .gone, .notAtPrompt: .notAtPrompt
        case .concealed: .refused(.concealed)
        case .sameColor: .refused(.sameColor)
        case .invisible: .refused(.invisible)
        case .lines: .refused(.lines)
        }
    }

    private static func state(_ status: BlockStatus) -> TerminalCore.BlockState {
        switch status {
        case .prompt: .prompt
        case .running: .running
        case let .finished(code, tookMs): .finished(code: code.map(Int.init), took: tookMs.map { .milliseconds($0) })
        }
    }

    private static func block(_ record: BlockRecord) -> CommandBlock {
        CommandBlock(
            id: record.id, start: Int(record.start), outputStart: record.outputStart.map(Int.init), end: Int(record.end),
            command: record.command, state: state(record.status))
    }
}
