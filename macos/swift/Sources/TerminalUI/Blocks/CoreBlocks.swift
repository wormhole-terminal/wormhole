// `CommandBlocks` over the real terminal's seam: each call is one call into Rust, whose engine holds the blocks (the
// view keeps only hover and the jump's cursor). The terminal is asked for each time: its source starts after the view is
// in a window, and a terminal that has none (the mock) shows no blocks. `palette` is the theme's colours when a command
// is put at the prompt, which the engine compares to tell hidden text from shown.

import Foundation
import TerminalCore

@MainActor
public final class CoreBlocks: CommandBlocks {
    private let terminal: @MainActor () -> (any TerminalBlocks)?
    private let palette: @MainActor () -> [UInt32]

    public init(terminal: @escaping @MainActor () -> (any TerminalBlocks)?, palette: @escaping @MainActor () -> [UInt32]) {
        (self.terminal, self.palette) = (terminal, palette)
    }

    public var spans: [BlockSpan] { terminal()?.blockSpans() ?? [] }
    public var displayOffset: Int { terminal()?.displayOffset ?? 0 }
    public var shellAtPrompt: Bool { terminal()?.shellAtPrompt ?? false }
    public var bottommostLine: Int { terminal()?.bottommostLine ?? 0 }
    public var topLine: Int { terminal()?.topLine ?? 0 }
    public func block(id: UInt64) -> CommandBlock? { terminal()?.block(id: id) }
    public func block(atLine line: Int) -> CommandBlock? { terminal()?.block(atLine: line) }
    public func newestRan() -> CommandBlock? { terminal()?.newestRanBlock() }

    public func promptLine(from line: Int, direction: PromptDirection) -> Int? {
        terminal()?.promptLine(from: line, direction: direction)
    }

    public func scrollLineToTop(_ line: Int) { terminal()?.scrollLineToTop(line) }
    public func scrollToBottom() { terminal()?.scrollToBottom() }
    public func selectLines(_ lines: ClosedRange<Int>) { terminal()?.selectLines(lines) }
    public func linesText(_ lines: ClosedRange<Int>) -> String { terminal()?.linesText(lines) ?? "" }

    public func putAtPrompt(_ block: CommandBlock) -> PutOutcome {
        terminal()?.putAtPrompt(id: block.id, palette: palette()) ?? .notAtPrompt
    }
}
