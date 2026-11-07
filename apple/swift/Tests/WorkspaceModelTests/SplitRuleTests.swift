import XCTest

@testable import WorkspaceModel

final class SplitRuleTests: XCTestCase {
    func testASplitOfOneIsNone() {
        XCTAssertEqual(SplitRules.normalised([1, nil, 2, 1]), [1, nil, nil, 1])
    }

    func testMembersGatherAtTheFirstOnesPlace() {
        // Core's `gather_splits`: 0 and 2 are one split, 1 and 3 another.
        XCTAssertEqual(SplitRules.gather([0, 1, 2, 3], [7, 8, 7, 8]), [0, 2, 1, 3])
        XCTAssertEqual(SplitRules.gather([3, 0, 1, 2], [nil, nil, 7, 7]), [3, 2, 0, 1])
    }

    func testPinsGoFirstEachBlockKeepingItsOrder() {
        // Core's `with_pins` case: 1 and 3 pinned.
        let pinned = [false, true, false, true, false]
        XCTAssertEqual(SplitRules.withPins([0, 1, 2, 3, 4], pinned), [1, 3, 0, 2, 4])
        XCTAssertEqual(SplitRules.withPins([4, 3, 2, 1, 0], pinned), [3, 1, 4, 2, 0])
    }

    func testPinningOneMemberPinsTheSplit() {
        XCTAssertEqual(SplitRules.pinned([true, false, false], [5, 5, nil]), [true, true, false])
    }

    func testTogetherRowsGetABracketAndSeparatedOnesAChain() {
        let ids: [Int?] = [1, 1, 1, nil]
        let together = SplitRules.marks(
            order: [0, 1, 2, 3], band: [nil, nil, nil, nil], shown: [true, true, true, true], ids: ids,
            selected: 1)
        XCTAssertEqual(together[0]?.bracket, .start)
        XCTAssertEqual(together[1]?.bracket, .mid)
        XCTAssertEqual(together[2]?.bracket, .end)
        XCTAssertEqual(together[0]?.onScreen, true)
        XCTAssertEqual(together[1]?.onScreen, false, "the selected one is not 'beside' itself")
        XCTAssertEqual(together[0]?.members, 3)
        XCTAssertNil(together[3])
        // A group boundary between members: chain marks, no bracket.
        let apart = SplitRules.marks(
            order: [0, 1], band: [nil, 0], shown: [true, true], ids: [2, 2], selected: nil)
        XCTAssertEqual(apart[0], SplitMarks(chain: true, members: 2))
    }

    func testTheMockSplitShowsABracketInTheListing() {
        let rows = SidebarListing.sections(for: MockWorkspace.state).flatMap(\.rows)
        XCTAssertEqual(rows.first { $0.id == 5 }?.splitMarks.bracket, .start)
        XCTAssertEqual(rows.first { $0.id == 6 }?.splitMarks.bracket, .end)
        XCTAssertEqual(rows.first { $0.id == 1 }?.splitMarks, SplitMarks())
    }

    func testPinnedSplitMovesAsOneAmongTheLoose() {
        var state = WorkspaceState(
            terminals: [
                TerminalSummary(id: 1, title: "a"), TerminalSummary(id: 2, title: "b"),
                TerminalSummary(id: 3, title: "c", isPinned: true, splitID: 9),
                TerminalSummary(id: 4, title: "d", splitID: 9),
            ])
        XCTAssertEqual(SidebarListing.sections(for: state).flatMap(\.rows).map(\.id), [3, 4, 1, 2])
        state.terminals[2].isPinned = false
        XCTAssertEqual(SidebarListing.sections(for: state).flatMap(\.rows).map(\.id), [1, 2, 3, 4])
    }
}
