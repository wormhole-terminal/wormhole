import XCTest

@testable import WorkspaceModel

final class SidebarListingTests: XCTestCase {
    private let mock = MockWorkspace.state

    private func ids(_ section: SidebarSection) -> [TerminalID] { section.rows.map(\.id) }

    func testPinnedFirstThenGroupsThenTheUngroupedRun() {
        let sections = SidebarListing.sections(for: mock)
        XCTAssertEqual(sections.map(\.id), ["pinned", "wormhole", "website", "infra", "ungrouped"])
        XCTAssertEqual(sections.map(\.title), ["Pinned", "Wormhole", "Website", "Infra", nil])
        XCTAssertEqual(ids(sections[0]), [3, 10, 12])
        XCTAssertEqual(ids(sections[1]), [1, 2, 4], "a pinned member leaves its group")
        XCTAssertEqual(ids(sections[2]), [5, 6, 7])
        XCTAssertEqual(ids(sections[4]), [11, 13])
    }

    func testACollapsedGroupKeepsItsHeadAndListsNoRows() {
        let infra = SidebarListing.sections(for: mock)[3]
        XCTAssertTrue(infra.isCollapsed)
        XCTAssertTrue(infra.rows.isEmpty)
        XCTAssertEqual(infra.terminalCount, 2)
        XCTAssertEqual(infra.alertCount, 0)
    }

    func testAlertCountsNeedsInputAndFailed() {
        let website = SidebarListing.sections(for: mock)[2]
        XCTAssertEqual(website.alertCount, 1)
        XCTAssertEqual(SidebarListing.sections(for: mock)[0].alertCount, 2)
    }

    func testNumbersCountTheFirstNineVisibleRows() {
        let rows = SidebarListing.sections(for: mock).flatMap(\.rows)
        XCTAssertEqual(rows.map(\.id), [3, 10, 12, 1, 2, 4, 5, 6, 7, 11, 13])
        XCTAssertEqual(rows.prefix(9).map(\.number), (1...9).map { $0 })
        XCTAssertEqual(rows.suffix(2).map(\.number), [nil, nil])
    }

    func testFoldingAGroupRenumbersTheRowsBelowIt() {
        let folded = reduce(mock, .toggleGroup("wormhole"))
        let rows = SidebarListing.sections(for: folded).flatMap(\.rows)
        XCTAssertEqual(rows.first { $0.id == 5 }?.number, 4)
        XCTAssertEqual(SidebarListing.sections(for: folded)[1].terminalCount, 3)
    }

    func testAnEmptyGroupKeepsItsHeadButEmptyPinnedAndUngroupedGoAway() {
        var state = mock
        state.terminals = state.terminals.filter { $0.id == 8 || $0.id == 9 }
        state.groups[2].isCollapsed = false
        state.groups[0].members = []
        let sections = SidebarListing.sections(for: state)
        XCTAssertEqual(sections.map(\.id), ["wormhole", "website", "infra"])
        XCTAssertTrue(sections[0].rows.isEmpty)
    }

    func testSearchMatchesTitleSubtitleAndBranchIgnoringCase() {
        func found(_ text: String) -> [TerminalID] {
            SidebarListing.sections(for: reduce(mock, .search(text))).flatMap(\.rows).map(\.id)
        }
        XCTAssertEqual(found("CARGO"), [2])
        XCTAssertEqual(found("/tmp"), [13])
        XCTAssertEqual(found("hero-copy"), [5, 7])
        XCTAssertEqual(found("  zsh "), [4], "surrounding spaces are ignored")
        XCTAssertEqual(found(""), SidebarListing.sections(for: mock).flatMap(\.rows).map(\.id))
        XCTAssertEqual(found("nothing like this"), [])
    }

    func testSearchHidesEmptySectionsAndNumbersWhatIsLeft() {
        let sections = SidebarListing.sections(for: reduce(mock, .search("hero")))
        XCTAssertEqual(sections.map(\.id), ["website"])
        XCTAssertEqual(sections[0].rows.map(\.number), [1, 2])
        XCTAssertTrue(SidebarListing.sections(for: reduce(mock, .search("zzz"))).isEmpty)
    }

    func testATerminalIsListedOnceEvenIfTwoGroupsClaimIt() {
        var state = mock
        state.groups[1].members.append(1)
        let rows = SidebarListing.sections(for: state).flatMap(\.rows).map(\.id)
        XCTAssertEqual(rows.filter { $0 == 1 }.count, 1)
    }

    func testMembersWithoutATerminalAreSkipped() {
        var state = mock
        state.groups[0].members.append(404)
        XCTAssertEqual(ids(SidebarListing.sections(for: state)[1]), [1, 2, 4])
    }
}
