import Foundation
import WorkspaceModel
import XCTest

@testable import WorkspaceBridge

/// The store over the real Rust workspace, with `/bin/cat` as the program of every terminal it opens: an action goes in,
/// a snapshot comes back, and the rules (order, selection after a close, numbers, wording) are the core's.
@MainActor
final class WorkspaceFFITests: XCTestCase {
    override func setUp() {
        setenv("WORMHOLE_DATA_DIR", "/tmp/Wormhole-swift-bridge-test", 1)
    }

    private func make(_ setup: WorkspaceSetup = WorkspaceSetup(program: "/bin/cat", home: "/Users/ref")) -> (WorkspaceStore, RustWorkspaceBackend) {
        let backend = RustWorkspaceBackend(setup: setup)
        return (WorkspaceStore(backend: backend), backend)
    }

    func testNewTerminalsAreRowsOfRealPrograms() {
        let (store, backend) = make()
        store.send(.newTerminal)
        store.send(.newTerminal)
        XCTAssertEqual(store.state.live, [1, 2])
        XCTAssertEqual(store.state.terminals.map(\.id), [1, 2])
        XCTAssertEqual(store.state.selected, 2, "a new terminal is selected")
        XCTAssertNotNil(backend.frameSource(for: 1), "the program started and has a source")
        XCTAssertEqual(store.state.sections.first?.rows.first?.number, 1)
        XCTAssertEqual(store.state.sections.first?.rows.first?.keycap, "⌘1")
    }

    func testRenamePinAndCloseAreTheCoresRules() {
        let (store, _) = make()
        (0..<3).forEach { _ in store.send(.newTerminal) }
        store.send(.rename(1, "build"))
        store.send(.togglePin(3))
        XCTAssertEqual(store.state.terminals.map(\.id), [3, 1, 2], "a pinned terminal leads")
        XCTAssertEqual(store.state.terminals.first?.title, "cat", "a terminal is named for what runs in it")
        XCTAssertEqual(store.state.terminals.first { $0.id == 1 }?.title, "build")
        store.send(.close(3))
        XCTAssertEqual(store.state.live, [1, 2])
        XCTAssertEqual(store.state.selected, 2, "the core picks the heir")
        XCTAssertEqual(store.send(.select(99)), .unknown)
    }

    func testGroupsFoldAndTheirRowsLeaveTheSnapshot() {
        let (store, _) = make()
        (0..<3).forEach { _ in store.send(.newTerminal) }
        store.send(.newGroup([1, 2]))
        let head = try? XCTUnwrap(store.state.groups.first)
        XCTAssertEqual(store.state.sections.first { $0.head != nil }?.rows.map(\.id), [1, 2])
        store.send(.toggleGroup(head?.id ?? 0))
        let folded = store.state.sections.first { $0.head != nil }
        XCTAssertEqual(folded?.rows.count, 0)
        XCTAssertEqual(folded?.terminalCount, 2)
        XCTAssertEqual(store.state.live, [1, 2, 3], "folded away, still alive")
        store.send(.renameGroup(head?.id ?? 0, "Work"))
        XCTAssertEqual(store.state.groups.first?.name, "Work")
    }

    func testANewTerminalInAGroupJoinsIt() {
        let (store, _) = make()
        store.send(.newTerminal)
        store.send(.newGroup([1]))
        let group = store.state.groups[0].id
        store.send(.newTerminalInGroup(group))
        XCTAssertEqual(store.state.sections.first { $0.head != nil }?.rows.map(\.id), [1, 2])
    }

    func testADropPlanIsAskedBeforeItIsLetGo() {
        let (store, _) = make()
        (0..<3).forEach { _ in store.send(.newTerminal) }
        let plan = store.dropPlan(dragged: 3, over: .row(1, .top))
        XCTAssertEqual(plan, .move(.top(before: .terminal(1)), leavesSplit: false, withMates: false))
        XCTAssertNil(store.dropPlan(dragged: 1, over: .row(1, .top)), "nothing would change")
        store.send(.dropTerminal(3, over: .row(1, .top)))
        XCTAssertEqual(store.state.terminals.map(\.id), [3, 1, 2])
    }

    func testASplitBesideJoinsOneCardAndLeavesIt() {
        let (store, _) = make()
        store.send(.newTerminal)
        store.send(.splitRight(1))
        let rows = store.state.sections.flatMap(\.rows)
        XCTAssertEqual(rows.map(\.terminal.splitID).compactMap { $0 }.count, 2)
        XCTAssertEqual(rows.map(\.splitMarks.bracket), [.start, .end])
    }

    func testTheProgramsOwnTitleReachesTheRow() async throws {
        // The program sets an OSC 2 title, then idles as `cat`.
        let (store, _) = make(WorkspaceSetup(program: "/bin/sh", arguments: ["-c", #"printf '\033]2;from the program\007'; exec cat"#]))
        store.send(.newTerminal)
        // The title arrives on the terminal's own thread and a wake; each change of the state is one look at it.
        for _ in 0..<20 where store.state.terminals.first?.title != "from the program" {
            let changed = expectation(description: "the state changed")
            withObservationTracking { _ = store.state } onChange: { changed.fulfill() }
            await fulfillment(of: [changed], timeout: 2)
        }
        XCTAssertEqual(store.state.terminals.first?.title, "from the program")
    }
}
