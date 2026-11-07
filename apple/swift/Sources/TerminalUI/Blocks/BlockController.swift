// Command blocks in the pane (`terminal_view/blocks.rs`): which finished block is hovered, where the last ⌘↑ / ⌘↓
// landed, and the Edit menu's actions on a block. The blocks themselves are `CommandBlocks`' (Rust's); this
// keeps only the view's state. What was copied never goes into a notice, only how much.

import AppKit
import Observation

@MainActor @Observable
public final class BlockController {
    public enum Action: Sendable { case selectOutput, copyOutput, copyCommand, putAtPrompt }

    /// A finished block under the pointer: tinted, with the readout.
    public private(set) var hovered: UInt64?
    /// The pointer is on the readout itself, which stays while it is.
    public var readoutHovered = false {
        didSet { if !readoutHovered && oldValue { hovered = nil } }
    }
    /// The block ⌘↑ / ⌘↓ last landed on: where the next jump starts and what the Edit menu acts on, until the
    /// view is scrolled or typed in.
    @ObservationIgnored private var cursor: UInt64?
    /// How far back the view was when the jump landed: a different offset later means the person scrolled or typed.
    @ObservationIgnored private var cursorOffset = 0

    @ObservationIgnored private let source: CommandBlocks
    @ObservationIgnored private let say: (Notice) -> Void
    @ObservationIgnored private let announce: (String) -> Void
    @ObservationIgnored private let write: (String) -> Void

    public init(
        source: CommandBlocks,
        say: @escaping (Notice) -> Void,
        announce: @escaping (String) -> Void = { _ in },
        write: @escaping (String) -> Void = BlockController.writeToPasteboard
    ) {
        (self.source, self.say, self.announce, self.write) = (source, say, announce, write)
    }

    public var spans: [BlockSpan] { source.spans }
    public var isAtPrompt: Bool { source.shellAtPrompt }
    /// How far the view is scrolled back into the history.
    public var displayOffset: Int { source.displayOffset }

    /// The pointer is over viewport `row` (nil: outside the grid): the finished block under it gets the tint.
    public func hover(row: Int?) {
        guard !readoutHovered else { return }
        hovered = row.flatMap { row in
            source.spans.first { $0.rows.contains(row) && $0.state.ran && isFinished($0.state) }?.id
        }
    }

    public func leave() { if !readoutHovered { hovered = nil } }

    /// The view scrolled or was typed in: the next jump starts from what is in view.
    public func forgetCursor() { cursor = nil }

    /// The terminal drew a frame: a view that is not where the last jump left it has been scrolled or typed in.
    public func noteView() {
        if cursor != nil, source.displayOffset != cursorOffset { forgetCursor() }
    }

    /// Block `id`, looked up now: a click may come a while after the readout showed, when its rows moved or went.
    public func block(id: UInt64) -> CommandBlock? { source.block(id: id) }

    /// The block under a viewport row, for the context menu.
    public func block(underRow row: Int) -> CommandBlock? {
        source.spans.first { $0.rows.contains(row) && $0.state.ran }.flatMap { source.block(id: $0.id) }
    }

    /// The block the actions act on: the one the last jump landed on, else the newest whose command ran.
    public var target: CommandBlock? {
        if let cursor, let block = source.block(id: cursor) { return block }
        return source.newestRan()
    }

    public func perform(_ action: Action, on block: CommandBlock? = nil) {
        guard let block = block ?? target else { return }
        switch action {
        case .selectOutput:
            if let lines = block.outputLines { source.selectLines(lines) }
        case .copyOutput:
            guard let lines = block.outputLines else { return }
            let output = source.linesText(lines)
            write(output)
            say(.copiedOutput(output))
        case .copyCommand:
            guard !block.command.isEmpty else { return }
            write(block.command)
            say(.copied(.command))
        case .putAtPrompt:
            guard !block.command.isEmpty else { return }
            if case let .refused(why) = source.putAtPrompt(block) { say(.failed(.putAtPrompt(why))) }
        }
    }

    /// ⌘↑ / ⌘↓: the previous or next prompt comes to the top of the view, as Terminal.app's marks do. From the
    /// live screen, up goes to the prompt before the current one; past the last prompt, down returns to the live
    /// screen. VoiceOver hears the block landed on.
    public func jumpPrompt(_ direction: PromptDirection) {
        let atBottom = source.displayOffset == 0
        // From the block the last jump landed on (a prompt near the bottom may not have been able to scroll to
        // the top), else the live prompt, else the top of the view (strictly: a prompt on the top row is passed).
        let from: Int? = if let cursor, let block = source.block(id: cursor) { block.start }
            else if atBottom { source.promptLine(from: source.bottommostLine + 1, direction: .up) }
            else { -source.displayOffset }
        guard let from else { return }
        if let line = source.promptLine(from: from, direction: direction) {
            source.scrollLineToTop(line)
            let block = source.block(atLine: line)
            cursor = block?.id
            cursorOffset = source.displayOffset
            if let block { announce(BlockWording.spoken(block)) }
        } else if direction == .down {
            if !atBottom { source.scrollToBottom() }
            cursor = nil
        }
    }

    private func isFinished(_ state: BlockState) -> Bool { if case .finished = state { true } else { false } }

    public nonisolated static func writeToPasteboard(_ text: String) {
        NSPasteboard.general.clearContents()
        NSPasteboard.general.setString(text, forType: .string)
    }
}
