import XCTest

@testable import WorkspaceModel

final class ReducerTests: XCTestCase {
    private let mock = MockWorkspace.state

    func testSelectKnownTerminalOnly() {
        XCTAssertEqual(reduce(mock, .select(6)).selected, 6)
        XCTAssertEqual(reduce(mock, .select(999)).selected, mock.selected)
    }

    func testNewTerminalAppendsSelectsAndClearsSearch() {
        var state = mock
        state.search = "zsh"
        let next = reduce(state, .newTerminal)
        XCTAssertEqual(next.terminals.last?.title, "Terminal 14")
        XCTAssertEqual(next.selected, 14)
        XCTAssertEqual(next.search, "")
        XCTAssertEqual(next.terminals.count, mock.terminals.count + 1)
        XCTAssertEqual(reduce(WorkspaceState(), .newTerminal).selected, 1)
    }

    func testClosingTheSelectedSelectsTheRowThatTakesItsPlace() {
        // Listed: pinned 3, 10, 12; Wormhole 1, 2, 4; Website 5, 6, 7; Infra 8, 9; loose 11, 13.
        var state = mock
        state.selected = 2
        XCTAssertEqual(reduce(state, .close(2)).selected, 4)
        state.selected = 13
        XCTAssertEqual(reduce(state, .close(13)).selected, 11, "the last one falls back to the row above")
    }

    func testClosingAnotherKeepsTheSelectionAndLeavesGroups() {
        let next = reduce(mock, .close(2))
        XCTAssertEqual(next.selected, 1)
        XCTAssertEqual(next.groups[0].members, [1, 3, 4])
        XCTAssertNil(next.terminals.first { $0.id == 2 })
    }

    func testClosingTheLastTerminalSelectsNothing() {
        let one = reduce(WorkspaceState(), .newTerminal)
        XCTAssertNil(reduce(one, .close(1)).selected)
    }

    func testClosingAnUnknownTerminalChangesNothing() {
        XCTAssertEqual(reduce(mock, .close(999)), mock)
    }

    func testToggles() {
        XCTAssertTrue(reduce(mock, .togglePin(1)).terminals[0].isPinned)
        XCTAssertFalse(reduce(mock, .togglePin(3)).terminals[2].isPinned)
        XCTAssertTrue(reduce(mock, .toggleGroup("wormhole")).groups[0].isCollapsed)
        XCTAssertFalse(reduce(mock, .toggleGroup("infra")).groups[2].isCollapsed)
        XCTAssertEqual(reduce(mock, .toggleGroup("nope")), mock)
        XCTAssertFalse(reduce(mock, .toggleSidebar).sidebarVisible)
        XCTAssertFalse(reduce(mock, .togglePanel).panelVisible)
    }

    func testShowingAPanelTabOpensThePanel() {
        var state = mock
        state.panelVisible = false
        let next = reduce(state, .showPanelTab(.notes))
        XCTAssertEqual(next.panelTab, .notes)
        XCTAssertTrue(next.panelVisible)
    }

    func testSearchStoresTheText() {
        XCTAssertEqual(reduce(mock, .search("cargo")).search, "cargo")
    }

    @MainActor
    func testStoreAppliesActions() {
        let store = WorkspaceStore(state: mock)
        store.send(.select(5))
        store.send(.toggleSidebar)
        XCTAssertEqual(store.state.selected, 5)
        XCTAssertFalse(store.state.sidebarVisible)
    }
}
