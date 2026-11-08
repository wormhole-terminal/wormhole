import XCTest

@testable import WorkspaceModel

/// The store over the sample backend: what it pulls, when, and what it hands on. The rules themselves are the core's
/// (`WorkspaceBridgeTests` runs them through the FFI).
@MainActor
final class StoreTests: XCTestCase {
    func testTheFirstPullIsTheWholeSampleWindow() {
        let store = WorkspaceStore(backend: MockWorkspaceBackend())
        var sample = MockWorkspace.state
        sample.panes = store.state.panes
        XCTAssertEqual(store.state, sample)
        XCTAssertEqual(store.state.panes.focused, 1, "the selected terminal fills the pane area")
        XCTAssertEqual(store.state.selected, 1)
        XCTAssertEqual(store.state.live.count, 13, "terminals folded away with their group still exist")
        XCTAssertEqual(store.state.terminals.count, 11, "but their rows are not listed")
    }

    func testAnActionIsAnswerByRevisionAndAChangeShowsInState() {
        let store = WorkspaceStore(backend: MockWorkspaceBackend())
        store.send(.togglePin(2))
        XCTAssertEqual(store.state.terminals.first { $0.id == 2 }?.isPinned, true)
        store.send(.toggleGroup(3))
        XCTAssertEqual(store.state.sections.first { $0.head?.id == 3 }?.rows.map(\.id), [8, 9], "opened")
        store.send(.newTerminal)
        XCTAssertEqual(store.state.selected, 14)
        XCTAssertEqual(store.state.live.last, 14)
    }

    func testChromeSwitchesComeFromTheChromeSnapshot() {
        let store = WorkspaceStore(backend: MockWorkspaceBackend())
        store.send(.showPanelTab(.history))
        store.send(.toggleSidebar)
        store.send(.showOverlay(.palette))
        XCTAssertEqual(store.state.panelTab, .history)
        XCTAssertFalse(store.state.sidebarVisible)
        XCTAssertEqual(store.state.overlay, .palette)
    }

    func testARefusalIsReturnedNotThrown() {
        let store = WorkspaceStore(backend: MockWorkspaceBackend())
        XCTAssertEqual(store.send(.select(999)), .unknown)
        XCTAssertEqual(store.send(.leaveSplit(1)), .unchanged)
    }

    func testANewTerminalInAGroupIsOneActionOfTheCore() {
        let backend = RecordingBackend()
        let store = WorkspaceStore(backend: backend)
        store.send(.newTerminalInGroup(7))
        XCTAssertEqual(backend.applied, [.newTerminalInGroup(7)])
    }

    func testTheOneTimerPullsTheWordsAgainWhenTheyMove() async throws {
        let backend = RecordingBackend()
        backend.refreshIn = .milliseconds(20)
        let store = WorkspaceStore(backend: backend)
        XCTAssertEqual(backend.sidebarPulls, 1)
        try await Task.sleep(for: .milliseconds(200))
        XCTAssertEqual(backend.sidebarPulls, 2, "one timer, armed once, and the words said nothing moves after it")
        _ = store
    }

    func testAWakeDrainsAndPullsOnlyWhatMoved() {
        let backend = RecordingBackend()
        let store = WorkspaceStore(backend: backend)
        backend.sidebarRevision += 1
        backend.onWake?()
        XCTAssertEqual(backend.drains, 1)
        XCTAssertEqual(backend.sidebarPulls, 2)
        XCTAssertEqual(backend.chromePulls, 1, "the chrome's number did not move")
        _ = store
    }
}

@MainActor
private final class RecordingBackend: WorkspaceBackend {
    var onWake: (@MainActor () -> Void)?
    var onNotice: (@MainActor (WorkspaceNotice) -> Void)?
    func terminals() -> TerminalsSnapshot { TerminalsSnapshot() }
    var live: [TerminalID] { [1] }
    var applied: [WorkspaceAction] = []
    var sidebarRevision: UInt64 = 1
    var sidebarPulls = 0, chromePulls = 0, drains = 0
    var refreshIn: Duration?

    func apply(_ action: WorkspaceAction) -> Refusal? { applied.append(action); return nil }
    func drain() { drains += 1 }
    func revisions() -> Revisions { Revisions(sidebar: sidebarRevision, chrome: 1) }
    func chrome() -> ChromeState { chromePulls += 1; return ChromeState() }
    func dropPlan(dragged: TerminalID, over: DropOver) -> DropPlan? { nil }

    func sidebar() -> SidebarSnapshot {
        sidebarPulls += 1
        defer { refreshIn = nil }
        return SidebarSnapshot(
            sections: [SidebarSection(id: "loose-1", rows: [SidebarRow(terminal: TerminalSummary(id: 1, title: "a"))])],
            selected: 1, refreshIn: refreshIn)
    }
}
