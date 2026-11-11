import Testing

@testable import TerminalUI

/// The adapters over the terminal's seam: lines are fetched again only when the stamp moves, and a view that is not where
/// the last jump left it forgets the jump.
@MainActor @Suite struct CoreSourcesTests {
    private final class FakeMinimap: TerminalMinimap, @unchecked Sendable {
        var stamp: UInt64 = 1
        var fetches = 0
        var scrolled: [Int] = []
        func minimapRead(fits: Int) -> MinimapRead { MinimapRead(total: 100, screen: 30, fits: fits, viewTop: 70, columns: 80, stamp: stamp) }
        func minimapLines() -> [[MinimapRun]] { fetches += 1; return [[MinimapRun(start: 0, end: 3, isText: true, cell: PackedCell())]] }
        func minimapFolds() -> [ClosedRange<Int>] { [72...90] }
        func scroll(toDisplayOffset offset: Int) { scrolled.append(offset) }
    }

    @Test func minimapLinesAreFetchedOnlyWhenTheStampMoves() {
        let fake = FakeMinimap()
        let minimap = CoreMinimap(terminal: { fake }, palette: { .spike })
        #expect(minimap.snapshot(fits: 50).lines.count == 1)
        #expect(minimap.snapshot(fits: 50).folds == [72...90], "a collapsed fold is a tick, read with each snapshot")
        _ = minimap.snapshot(fits: 50)
        #expect(fake.fetches == 1)
        fake.stamp += 1
        _ = minimap.snapshot(fits: 50)
        #expect(fake.fetches == 2)
        #expect(minimap.snapshot(fits: 50).geometry == MinimapGeometry(total: 100, screen: 30, fits: 50, viewTop: 70))
        minimap.scroll(toDisplayOffset: 12)
        #expect(fake.scrolled == [12])
    }

    @Test func withoutATerminalTheMinimapIsEmpty() {
        let minimap = CoreMinimap(terminal: { nil }, palette: { .spike })
        #expect(minimap.snapshot(fits: 10).lines.isEmpty)
    }

    @Test func aViewMovedAwayFromTheLastJumpForgetsIt() {
        let blocks = MockBlocks()
        let controller = BlockController(source: blocks, say: { _ in })
        controller.jumpPrompt(.up)
        controller.jumpPrompt(.up)
        #expect(controller.target?.id == 2, "the jump's block, not the newest")
        controller.noteView()
        #expect(controller.target?.id == 2, "the view is where the jump left it")
        blocks.displayOffset += 5
        controller.noteView()
        #expect(controller.target?.id == 3, "scrolled since: the newest block that ran")
    }
}
