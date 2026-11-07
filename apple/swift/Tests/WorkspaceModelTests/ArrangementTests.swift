import XCTest

@testable import WorkspaceModel

final class ArrangementTests: XCTestCase {
    private let mock = MockWorkspace.state

    private func members(_ state: WorkspaceState, _ group: String) -> [TerminalID] {
        state.groups.first { $0.id == group }?.members ?? []
    }

    func testRenameKeepsTheOldTitleForABlankOne() {
        XCTAssertEqual(reduce(mock, .rename(4, "  build box ")).terminals[3].title, "build box")
        XCTAssertEqual(reduce(mock, .rename(4, "   ")).terminals[3].title, "zsh")
        XCTAssertEqual(reduce(mock, .rename(99, "x")), mock)
    }

    func testNewGroupAppendsAnEmptyHeadWithAFreshId() {
        let once = reduce(mock, .newGroup(" Research "))
        XCTAssertEqual(once.groups.last?.name, "Research")
        XCTAssertEqual(once.groups.last?.members, [])
        let twice = reduce(once, .newGroup(""))
        XCTAssertEqual(Set(twice.groups.map(\.id)).count, 5)
        XCTAssertEqual(twice.groups.last?.name, "Group 5")
        XCTAssertEqual(SidebarListing.sections(for: twice).last?.title, "Group 5")
    }

    func testAddToGroupMovesTheTerminalAndDropsTheGroupItLeftEmpty() {
        let moved = reduce(mock, .addToGroup(11, "website"))
        XCTAssertEqual(members(moved, "website"), [5, 6, 7, 11])
        let drained = reduce(reduce(mock, .addToGroup(8, "wormhole")), .addToGroup(9, "wormhole"))
        XCTAssertFalse(drained.groups.contains { $0.id == "infra" }, "an emptied unpinned group goes")
        var pinned = mock
        pinned.groups[2].isPinned = true
        XCTAssertTrue(reduce(reduce(pinned, .addToGroup(8, "wormhole")), .addToGroup(9, "wormhole")).groups
            .contains { $0.id == "infra" })
        XCTAssertEqual(reduce(mock, .addToGroup(11, "nope")), mock)
    }

    func testMovingWithinAGroupLandsBeforeTheNamedTerminal() {
        let moved = reduce(mock, .moveTerminal(4, to: .group("wormhole", before: 1)))
        XCTAssertEqual(members(moved, "wormhole"), [4, 1, 2, 3])
        // Pins still lead the group as listed.
        XCTAssertEqual(SidebarListing.sections(for: moved)[1].rows.map(\.id), [3, 4, 1, 2])
    }

    func testMovingATerminalOutOfItsGroupPutsItAmongTheLoose() {
        let moved = reduce(mock, .moveTerminal(5, to: .top(before: .terminal(13))))
        XCTAssertEqual(members(moved, "website"), [6, 7])
        XCTAssertEqual(SidebarListing.sections(for: moved).flatMap(\.rows).map(\.id).suffix(2), [5, 13])
    }

    func testMovingAGroupReordersTheTopLevel() {
        let moved = reduce(mock, .moveGroup("infra", before: .group("wormhole")))
        XCTAssertEqual(SidebarListing.sections(for: moved).map(\.id).prefix(3), ["loose-10", "infra", "wormhole"])
        XCTAssertEqual(reduce(mock, .moveGroup("infra", before: .group("infra"))), mock)
    }

    func testAnUnpinnedTerminalDroppedAbovePinsStaysBelowThem() {
        let state = WorkspaceState(
            terminals: [
                TerminalSummary(id: 1, title: "a"), TerminalSummary(id: 2, title: "b", isPinned: true),
                TerminalSummary(id: 3, title: "c"),
            ])
        let moved = reduce(state, .moveTerminal(3, to: .top(before: .terminal(1))))
        XCTAssertEqual(SidebarListing.sections(for: moved).flatMap(\.rows).map(\.id), [2, 3, 1])
    }

    func testSplitRightJoinsTheAnchorsSplitAndGroup() {
        let split = reduce(mock, .splitRight(1))
        XCTAssertEqual(split.terminals.last?.id, 14)
        XCTAssertEqual(split.selected, 14)
        XCTAssertEqual(split.terminals.last?.splitID, split.terminals[0].splitID)
        XCTAssertEqual(members(split, "wormhole"), [1, 14, 2, 3, 4])
        let beside = reduce(mock, .splitRight(5))
        XCTAssertEqual(members(beside, "website"), [5, 6, 14, 7], "after the split's last member")
        XCTAssertEqual(beside.terminals.last?.splitID, 1)
    }
}
