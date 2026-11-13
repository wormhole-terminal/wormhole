import WorkspaceModel
import XCTest

@testable import WorkspaceBridge

/// The status bar's counts and detail through the real core.
final class StatusRulesFFITests: XCTestCase {
    private func terminal(_ id: Int, _ activity: Activity, since: UInt64? = nil) -> TerminalSummary {
        var summary = TerminalSummary(id: id, title: "t\(id)", activity: activity)
        summary.since = since
        return summary
    }

    func testTheLongestWaitingIsTheButtonAndTheRestAreCountedInTheBarsOrder() {
        let counts = CoreStatusRules().counts(of: [
            terminal(1, .working, since: 1), terminal(2, .needsInput, since: 900), terminal(3, .idle),
            terminal(4, .needsInput, since: 100), terminal(5, .failed, since: 5), terminal(6, .working, since: 2),
        ])
        XCTAssertEqual(counts.needsYou, [3, 1], "waiting since 100 before 900")
        XCTAssertEqual(
            counts.rest,
            [
                StatusCounts.Item(activity: .needsInput, count: 1, terminal: 1), StatusCounts.Item(activity: .working, count: 2, terminal: 0),
                StatusCounts.Item(activity: .failed, count: 1, terminal: 4),
            ])
    }

    func testTheWordsGoFirstThenTheSize() {
        let rules = CoreStatusRules()
        XCTAssertEqual(rules.detail(windowWidth: 1400), .words)
        XCTAssertEqual(rules.detail(windowWidth: 900), .numbers)
        XCTAssertEqual(rules.detail(windowWidth: 500), .lean)
    }
}
