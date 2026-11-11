import XCTest

@testable import WorkspaceModel

/// What the quit question counts and the status bar says follow where each terminal's program runs.
final class HostingTests: XCTestCase {
    func testOnlyAHostThatIsStillThereSurvivesTheQuit() {
        XCTAssertTrue(HostingState.persistent(hostID: "a").survivesQuit)
        XCTAssertTrue(HostingState.detached(hostID: "a").survivesQuit, "let go of on the way out")
        XCTAssertFalse(HostingState.local.survivesQuit)
        XCTAssertFalse(HostingState.lost(hostID: "a").survivesQuit, "its host took the program along")
        XCTAssertNil(HostingState.local.hostID)
        XCTAssertEqual(HostingState.lost(hostID: "a").hostID, "a")
    }

    func testTheQuitCountsSplitWhatRunsByWhereItRuns() {
        let hosting: (TerminalID) -> HostingState? = { id in
            switch id {
            case 1, 2: .persistent(hostID: "h\(id)")
            case 3: .local
            default: .lost(hostID: "gone")
            }
        }
        let counts = HostedCounts(running: [1, 3, 4], all: [1, 2, 3, 4], hosting: hosting)
        XCTAssertEqual(counts, HostedCounts(hostedRunning: 1, localRunning: 2, anyHosted: true))
    }

    func testWithoutPersistenceEverythingThatRunsWouldEnd() {
        let counts = HostedCounts(running: [1, 2], all: [1, 2, 3], hosting: { _ in nil })
        XCTAssertEqual(counts, HostedCounts(hostedRunning: 0, localRunning: 2, anyHosted: false))
    }

    func testTheSampleBackendHasNoHosts() async {
        let store = await MainActor.run { WorkspaceStore(backend: MockWorkspaceBackend()) }
        let state = await MainActor.run { store.hosting(of: 1) }
        XCTAssertNil(state)
    }
}
