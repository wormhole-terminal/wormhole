import XCTest

@testable import WorkspaceModel

final class SidebarListingTests: XCTestCase {
    private let mock = MockWorkspace.state

    private func ids(_ section: SidebarSection) -> [TerminalID] { section.rows.map(\.id) }
    private func listed(_ state: WorkspaceState) -> [TerminalID] {
        SidebarListing.sections(for: state).flatMap(\.rows).map(\.id)
    }

    func testPinnedLooseFirstThenTheArrangementWithPinsAtTheTopOfTheirGroup() {
        let sections = SidebarListing.sections(for: mock)
        XCTAssertEqual(sections.map(\.id), ["loose-10", "wormhole", "website", "loose-11", "infra", "loose-13"])
        XCTAssertEqual(sections.map(\.title), [nil, "Wormhole", "Website", nil, "Infra", nil])
        XCTAssertEqual(ids(sections[0]), [10, 12], "pinned loose terminals lead the list")
        XCTAssertEqual(ids(sections[1]), [3, 1, 2, 4], "a pinned member stays in its group, on top")
        XCTAssertEqual(ids(sections[3]), [11])
        XCTAssertEqual(sections[1].group?.color, .blue)
    }

    func testWithoutAnArrangementLooseTerminalsComeBeforeGroups() {
        var state = mock
        state.arrangement = []
        XCTAssertEqual(SidebarListing.sections(for: state).map(\.id), ["loose-10", "wormhole", "website", "infra"])
        XCTAssertEqual(listed(state).prefix(4), [10, 12, 11, 13])
    }

    func testACollapsedGroupKeepsItsHeadAndListsNoRows() {
        let infra = SidebarListing.sections(for: mock)[4]
        XCTAssertTrue(infra.isCollapsed)
        XCTAssertTrue(infra.rows.isEmpty)
        XCTAssertEqual(infra.terminalCount, 2)
    }

    func testAlertCountsNeedsInputAndFailed() {
        let sections = SidebarListing.sections(for: mock)
        XCTAssertEqual(sections[0].alertCount, 1)
        XCTAssertEqual(sections[1].alertCount, 1)
        XCTAssertEqual(sections[2].alertCount, 1)
    }

    func testNumbersCountTheFirstNineVisibleRows() {
        let rows = SidebarListing.sections(for: mock).flatMap(\.rows)
        XCTAssertEqual(rows.map(\.id), [10, 12, 3, 1, 2, 4, 5, 6, 7, 11, 13])
        XCTAssertEqual(rows.prefix(9).map(\.number), (1...9).map { $0 })
        XCTAssertEqual(rows.suffix(2).map(\.number), [nil, nil])
    }

    func testFoldingAGroupRenumbersTheRowsBelowIt() {
        let folded = reduce(mock, .toggleGroup("wormhole"))
        let rows = SidebarListing.sections(for: folded).flatMap(\.rows)
        XCTAssertEqual(rows.first { $0.id == 5 }?.number, 3)
    }

    func testAnEmptyGroupKeepsItsHeadButEmptyLooseRunsGoAway() {
        var state = mock
        state.terminals = state.terminals.filter { $0.id == 8 || $0.id == 9 }
        state.groups[2].isCollapsed = false
        state.groups[0].members = []
        let sections = SidebarListing.sections(for: state)
        XCTAssertEqual(sections.map(\.id), ["wormhole", "website", "infra"])
        XCTAssertTrue(sections[0].rows.isEmpty)
    }

    func testSearchMatchesTitleSubtitleAndBranchIgnoringCase() {
        func found(_ text: String) -> [TerminalID] { listed(reduce(mock, .search(text))) }
        XCTAssertEqual(found("CARGO"), [2])
        XCTAssertEqual(found("release/1"), [12])
        XCTAssertEqual(found("hero-copy"), [5, 6, 7])
        XCTAssertEqual(found("  zsh "), [4], "surrounding spaces are ignored")
        XCTAssertEqual(found(""), listed(mock))
        XCTAssertEqual(found("nothing like this"), [])
    }

    func testSearchHidesEmptySectionsAndNumbersWhatIsLeft() {
        let sections = SidebarListing.sections(for: reduce(mock, .search("hero")))
        XCTAssertEqual(sections.map(\.id), ["website"])
        XCTAssertEqual(sections[0].rows.map(\.number), [1, 2, 3])
        XCTAssertTrue(SidebarListing.sections(for: reduce(mock, .search("zzz"))).isEmpty)
    }

    func testATerminalIsListedOnceEvenIfTwoGroupsClaimIt() {
        var state = mock
        state.groups[1].members.append(1)
        XCTAssertEqual(listed(state).filter { $0 == 1 }.count, 1)
    }

    func testMembersWithoutATerminalAreSkipped() {
        var state = mock
        state.groups[0].members.append(404)
        XCTAssertEqual(ids(SidebarListing.sections(for: state)[1]), [3, 1, 2, 4])
    }

    func testTheSavedOrderIgnoresWhatIsGoneAndAppendsWhatIsNew() {
        var state = mock
        state.arrangement = [.terminal(404), .group("infra"), .terminal(1), .group("wormhole")]
        // 1 is in a group now, 404 is gone: infra, wormhole, then the unknown (website), loose after pins.
        XCTAssertEqual(
            SidebarListing.sections(for: state).map(\.id),
            ["loose-10", "infra", "wormhole", "loose-11", "website"])
    }
}
