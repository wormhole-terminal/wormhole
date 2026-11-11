#if WITH_UNIFFI
import Testing
import TerminalCore

@testable import TerminalMetal

/// Find, command blocks and the minimap on the real engine through the view's source (`FrameSource.scrollback`): the
/// search covers the history, the matches are painted by the view while the bar is open, the marks give blocks.
@MainActor @Suite struct ScrollbackTests {
    private static let long = "i=1; while [ $i -le 60 ]; do printf 'line %02d needle\\r\\n' $i; i=$((i+1)); done; printf 'END\\r\\n'"
    private static let marked =
        "printf '\\033]133;A\\033\\\\$ \\033]133;B\\033\\\\ls\\r\\n\\033]133;C\\033\\\\out\\r\\n\\033]133;D;2\\033\\\\'; "
        + "printf '\\033]133;A\\033\\\\$ \\033]133;B\\033\\\\'"

    @Test func aSearchCoversTheWholeHistoryAndTheViewPaintsTheCurrentMatch() async throws {
        let live = try LiveHarness(Self.long)
        #expect(await live.wait(for: "END"))
        let finder = try #require(live.view.source.scrollback)
        let found = finder.find("needle", matchCase: false, wholeWord: false, regex: false)
        #expect(found.count == 60)
        live.view.findActive = true
        finder.findReveal(match: 10)
        live.view.drawFrame(force: true)
        let spans = finder.findSpans()
        #expect(spans.filter(\.isCurrent).count == 1)
        #expect(spans.allSatisfy { $0.last - $0.first == 5 })
        #expect(!finder.findTicks().isEmpty)
    }

    @Test func blocksComeFromTheShellsMarksAndTheMinimapReadsTheHistory() async throws {
        let live = try LiveHarness(Self.marked, rows: 12)
        #expect(await live.wait(for: "out"))
        let scrollback = try #require(live.view.source.scrollback)
        for _ in 0..<100 where scrollback.blockSpans().count < 2 { live.view.drawFrame(force: true) }
        let spans = scrollback.blockSpans()
        #expect(spans.count == 2)
        let first = try #require(spans.first)
        guard case let .finished(code, _) = first.state else { Issue.record("not finished: \(first.state)"); return }
        #expect(code == 2)
        #expect(spans.last?.state == .prompt)
        let block = try #require(scrollback.block(id: first.id))
        #expect(block.command == "ls")
        let output = try #require(block.outputLines)
        #expect(scrollback.linesText(output).contains("out"))
        let read = scrollback.minimapRead(fits: 50)
        #expect(read.screen == 12)
        #expect(!scrollback.minimapLines().flatMap { $0 }.isEmpty)
    }
}
#endif
