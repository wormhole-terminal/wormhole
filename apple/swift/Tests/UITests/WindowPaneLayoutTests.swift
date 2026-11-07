import PanesModel
import Testing
import WorkspaceModel

@testable import WindowUI

/// The window's pane layouts against the workspace: a split names its new terminal, a drop joins, the selection is the focus.
@MainActor
@Suite struct WindowPaneLayoutTests {
    private static func state(_ ids: [Int], selected: Int, split: [Int] = []) -> WorkspaceState {
        let terminals = ids.map { TerminalSummary(id: $0, title: "t\($0)", splitID: split.contains($0) ? 1 : nil) }
        return WorkspaceState(terminals: terminals, selected: selected)
    }

    @Test func aLoneTerminalIsASinglePane() {
        let panes = PaneLayouts().panes(in: Self.state([1, 2], selected: 2))
        #expect(panes?.panes == [2])
    }

    @Test func splittingRightThenDownStacksTheNewPaneUnderTheFocusedOne() throws {
        var layouts = PaneLayouts()
        var state = Self.state([1], selected: 1)
        let first = layouts.split(.row, in: state)
        state = reduce(state, try #require(first))
        let second = layouts.split(.column, in: state)
        state = reduce(state, try #require(second))
        let panes = try #require(layouts.panes(in: state))
        #expect(panes.panes.sorted() == [1, 2, 3])
        guard case .split(.row, let children) = panes.tree.root, children.count == 2,
            case .split(.column, let stacked) = children[1].node
        else { Issue.record("not a row of a pane and a column: \(panes.tree.root)"); return }
        #expect(stacked.count == 2)
    }

    @Test func aPaneDroppedFromTheListJoinsTheSplit() throws {
        var layouts = PaneLayouts()
        var state = Self.state([1, 2, 3], selected: 1)
        var panes = try #require(layouts.panes(in: state))
        try panes.join(3, beside: 1, zone: .right, room: PaneRoom(0, 0))
        for action in layouts.store(panes, in: state) { state = reduce(state, action) }
        #expect(state.terminals.first { $0.id == 3 }?.splitID == state.terminals.first { $0.id == 1 }?.splitID)
        #expect(layouts.panes(in: state)?.panes.sorted() == [1, 3])
        #expect(state.selected == 3)
    }

    @Test func focusingAPaneSelectsItsTerminal() throws {
        var layouts = PaneLayouts()
        var state = Self.state([1, 2], selected: 1, split: [1, 2])
        var panes = try #require(layouts.panes(in: state))
        panes.focus(2)
        for action in layouts.store(panes, in: state) { state = reduce(state, action) }
        #expect(state.selected == 2)
    }

    @Test func leavingTheSplitLeavesNoSplitOfOne() {
        var state = Self.state([1, 2], selected: 1, split: [1, 2])
        state = reduce(state, .leaveSplit(2))
        #expect(state.terminals.allSatisfy { $0.splitID == nil })
    }
}

@MainActor
@Suite struct WindowHubPaneTests {
    @Test func theHubSplitsRightThenDown() {
        let hub = WindowHub(store: WorkspaceStore(state: MockWorkspace.state), terminals: TerminalViews { _ in nil })
        hub.split(.row)
        hub.split(.column)
        #expect(hub.paneState.contains("panes=row(1,col("), "\(hub.paneState)")
    }
}
