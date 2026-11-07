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
        XCTAssertEqual(state { $0.activity = .working }, .running, "the core says working while a command runs")
        XCTAssertEqual(state { $0.activity = .finished }, .shell, "only an agent finishes")
        XCTAssertEqual(state { $0.activity = .failed }, .failed)
        XCTAssertEqual(state { $0.unread = true }, .unread)
        XCTAssertEqual(state { $0.activity = .exited }, .shell)
        XCTAssertEqual(state { _ in }, .shell)
    }

    func testTheBellSpeaksOnlyWhenNothingLouderDoes() {
        XCTAssertEqual(state { $0.hasBell = true }, .bell)
        XCTAssertEqual(state { $0.hasBell = true; $0.activity = .working }, .bell)
        XCTAssertEqual(state { $0.activity = .needsInput }, .bell, "a plain terminal asks only by its bell")
        XCTAssertEqual(state { $0.hasBell = true; $0.unread = true }, .unread)
        XCTAssertEqual(state { $0.hasBell = true; $0.agent = .codex; $0.activity = .needsInput }, .needsYou)
    }
}
