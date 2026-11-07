import PanesModel
import Testing
import WorkspaceModel

@testable import WindowUI

/// The window's pane layouts against the workspace: a split names its new terminal, a drop joins, the selection is the focus.
@MainActor
@Suite struct WindowPaneLayoutTests {
    private static func hub(_ ids: [Int], selected: Int, split: [Int] = []) -> WindowHub {
        WindowHub(store: WorkspaceStore(backend: SplittingBackend(ids, selected: selected, split: split)), terminals: TerminalViews { _ in nil })
    }

    @Test func aLoneTerminalIsASinglePane() {
        let hub = Self.hub([1, 2], selected: 2)
        #expect(hub.layouts.panes(in: hub.state)?.panes == [2])
    }

    @Test func splittingRightThenDownStacksTheNewPaneUnderTheFocusedOne() throws {
        let hub = Self.hub([1], selected: 1)
        hub.split(.row)
        hub.split(.column)
        let panes = try #require(hub.layouts.panes(in: hub.state))
        #expect(panes.panes.sorted() == [1, 2, 3])
        guard case .split(.row, let children) = panes.tree.root, children.count == 2,
            case .split(.column, let stacked) = children[1].node
        else { Issue.record("not a row of a pane and a column: \(panes.tree.root)"); return }
        #expect(stacked.count == 2)
    }

    @Test func aPaneDroppedFromTheListJoinsTheSplit() throws {
        let hub = Self.hub([1, 2, 3], selected: 1)
        var panes = try #require(hub.layouts.panes(in: hub.state))
        try panes.join(3, beside: 1, zone: .right, room: PaneRoom(0, 0))
        hub.commit(panes)
        let terminals = hub.state.terminals
        #expect(terminals.first { $0.id == 3 }?.splitID == terminals.first { $0.id == 1 }?.splitID)
        #expect(hub.layouts.panes(in: hub.state)?.panes.sorted() == [1, 3])
        #expect(hub.state.selected == 3)
    }

    @Test func focusingAPaneSelectsItsTerminal() throws {
        let hub = Self.hub([1, 2], selected: 1, split: [1, 2])
        var panes = try #require(hub.layouts.panes(in: hub.state))
        panes.focus(2)
        hub.commit(panes)
        #expect(hub.state.selected == 2)
    }

    @Test func theActionsOfAChangeAreTheJoinsThenTheFocus() throws {
        let hub = Self.hub([1, 2, 3], selected: 1)
        var panes = try #require(hub.layouts.panes(in: hub.state))
        try panes.join(3, beside: 1, zone: .right, room: PaneRoom(0, 0))
        #expect(hub.layouts.actions(for: panes, in: hub.state) == [.joinSplit(3, with: 1), .select(3)])
    }

    @Test func theHubSplitsRightThenDown() {
        let hub = Self.hub([1], selected: 1)
        hub.split(.row)
        hub.split(.column)
        #expect(hub.paneState.contains("panes=row(1,col("), "\(hub.paneState)")
    }

    @Test func leavingTheSplitLeavesNoSplitOfOne() {
        let hub = Self.hub([1, 2], selected: 1, split: [1, 2])
        hub.send(.leaveSplit(2))
        #expect(hub.layouts.panes(in: hub.state)?.panes == [1])
    }
}
