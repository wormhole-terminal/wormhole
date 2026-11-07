import XCTest
@testable import SidebarUI
@testable import WorkspaceModel

final class SidebarTileTests: XCTestCase {
    private func state(_ configure: (inout TerminalSummary) -> Void) -> TileState {
        var terminal = TerminalSummary(id: 1, title: "t")
        configure(&terminal)
        return TileState(terminal)
    }

    func testAnAgentsStateIsToldInItsTile() {
        XCTAssertEqual(state { $0.agent = .claude; $0.activity = .working }, .working)
        XCTAssertEqual(state { $0.agent = .claude; $0.activity = .needsInput }, .needsYou)
        XCTAssertEqual(state { $0.agent = .claude; $0.activity = .finished }, .done)
        XCTAssertEqual(state { $0.agent = .claude; $0.activity = .failed }, .failed)
        XCTAssertEqual(state { $0.agent = .claude }, .shell, "an idle agent shows nothing round its tile")
    }

    func testAPlainTerminalSaysOnlyWhatItHasToSay() {
        XCTAssertEqual(state { $0.activity = .working }, .shell, "a running command is no light: the line says it")
        XCTAssertEqual(state { $0.runningCommand = "make" }, .running)
        XCTAssertEqual(state { $0.activity = .finished }, .shell, "only an agent finishes")
        XCTAssertEqual(state { $0.activity = .failed }, .failed)
        XCTAssertEqual(state { $0.lastFailure = CommandFailure(exitCode: 2, at: Date()) }, .failed)
        XCTAssertEqual(state { $0.unread = 3 }, .unread)
        XCTAssertEqual(state { _ in }, .shell)
    }

    func testTheBellSpeaksOnlyWhenNothingLouderDoes() {
        XCTAssertEqual(state { $0.hasBell = true }, .bell)
        XCTAssertEqual(state { $0.hasBell = true; $0.runningCommand = "make" }, .bell)
        XCTAssertEqual(state { $0.hasBell = true; $0.unread = 1 }, .unread)
        XCTAssertEqual(state { $0.hasBell = true; $0.agent = .codex; $0.activity = .needsInput }, .needsYou)
    }

    func testTheOrderIsCoresUrgencyOrder() {
        let loudestFirst: [TileState] = [.needsYou, .failed, .working, .bell, .unread, .running, .done, .shell]
        XCTAssertEqual(loudestFirst.shuffled().sorted(), loudestFirst)
        XCTAssertTrue(TileState.needsYou.isAlert && TileState.failed.isAlert && !TileState.working.isAlert)
    }

    func testOneTimerIsArmedForTheNearestWordsToChange() {
        let now = Date(timeIntervalSinceReferenceDate: 1000)
        let working = TerminalSummary(id: 1, title: "a", activity: .working, agent: .claude, activitySince: now.addingTimeInterval(-70))
        let command = TerminalSummary(id: 2, title: "b", runningCommand: "make", runningSince: now.addingTimeInterval(-3.5))
        let quiet = TerminalSummary(id: 3, title: "c")
        XCTAssertEqual(SidebarClock.nextChange(of: [quiet], now: now), nil, "a list with nothing that moves sleeps")
        XCTAssertEqual(try XCTUnwrap(SidebarClock.nextChange(of: [working], now: now)), now.addingTimeInterval(50))
        XCTAssertEqual(try XCTUnwrap(SidebarClock.nextChange(of: [working, command, quiet], now: now)), now.addingTimeInterval(0.5), "the command's second comes first")
    }
}
