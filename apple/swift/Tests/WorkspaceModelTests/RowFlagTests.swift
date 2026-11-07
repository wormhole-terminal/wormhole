import XCTest

@testable import WorkspaceModel

final class RowFlagTests: XCTestCase {
    private let mock = MockWorkspace.state

    private func rows(_ state: WorkspaceState) -> [SidebarRow] { SidebarListing.sections(for: state).flatMap(\.rows) }

    func testTheFlagsToggleAndToggleBack() {
        let hidden = reduce(mock, .toggleAgentHidden(1))
        XCTAssertEqual(hidden.terminals[0].isHiddenFromAgents, true)
        XCTAssertEqual(reduce(hidden, .toggleAgentHidden(1)), mock)
        XCTAssertEqual(reduce(mock, .toggleForgetText(1)).terminals[0].forgetsText, true)
        XCTAssertEqual(reduce(mock, .toggleForgetText(99)), mock)
    }

    func testTheMockWearsEachMark() {
        let byID = Dictionary(uniqueKeysWithValues: rows(mock).map { ($0.id, $0.terminal) })
        XCTAssertEqual(byID[3]?.worktreeOf, "~/Projects/terminal/wormhole")
        XCTAssertEqual(byID[13]?.forgetsText, true)
        XCTAssertEqual(mock.terminals.first { $0.id == 8 }?.isHiddenFromAgents, true)
    }

    func testARepositorysChangesShowOnTheTopmostRowOnly() {
        let shown = Dictionary(uniqueKeysWithValues: rows(mock).map { ($0.id, $0.showsChanges) })
        XCTAssertEqual(shown[5], true)
        XCTAssertEqual(shown[6], false)
        XCTAssertEqual(shown[7], false)
        XCTAssertEqual(shown[3], true, "no repository: each terminal's own count")
        XCTAssertEqual(shown[4], false, "nothing changed, nothing shown")
    }

    func testFoldingTheTopRowAwayLetsTheNextOneShowTheCount() {
        var state = mock
        state.groups[1].members = [6, 7]
        state.terminals[3].repoID = "web"  // 4, in another group, above 6
        state.terminals[3].changeCount = 4
        let shown = Dictionary(uniqueKeysWithValues: rows(state).map { ($0.id, $0.showsChanges) })
        XCTAssertEqual(shown[4], true)
        XCTAssertEqual(shown[6], false)
        // With the first group folded its rows are not on screen, so the next row of the repository is topmost.
        let folded = reduce(state, .toggleGroup("wormhole"))
        let after = Dictionary(uniqueKeysWithValues: rows(folded).map { ($0.id, $0.showsChanges) })
        XCTAssertEqual(after[6], true)
    }
}
