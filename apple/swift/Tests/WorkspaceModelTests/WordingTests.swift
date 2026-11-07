import XCTest

@testable import WorkspaceModel

final class WordingTests: XCTestCase {
    private let now = Date(timeIntervalSince1970: 1_000_000)
    private func ago(_ seconds: TimeInterval) -> Date { now.addingTimeInterval(-seconds) }
    private func line(_ terminal: TerminalSummary, home: String? = nil) -> SecondLine {
        SidebarWording.secondLine(for: terminal, now: now, home: home)
    }

    // The cases of core's `an_agents_second_line_says_what_it_does` and `times_read_short`.
    func testAnAgentsSecondLineSaysWhatItDoes() {
        let agent = { (activity: Activity, message: String?, since: TimeInterval) in
            self.line(TerminalSummary(
                id: 1, title: "t", activity: activity, agent: .claude, message: message, activitySince: self.ago(since)))
        }
        XCTAssertEqual(agent(.needsInput, "Approve cargo test?", 60).text, "Needs input · Approve cargo test?")
        XCTAssertEqual(agent(.needsInput, "  ", 60).text, "Needs input")
        XCTAssertEqual(agent(.needsInput, nil, 60).tone, .attention)
        XCTAssertEqual(agent(.working, "Editing sidebar.rs…", 180).text, "Editing sidebar.rs…")
        XCTAssertEqual(agent(.working, nil, 180).text, "Working · 3m")
        XCTAssertEqual(agent(.working, nil, 20).text, "Working · <1m")
        XCTAssertEqual(agent(.finished, nil, 720).text, "Finished · 12m ago")
        XCTAssertEqual(agent(.finished, nil, 720).tone, .done)
        XCTAssertEqual(agent(.finished, nil, 0).text, "Finished · just now")
        XCTAssertEqual(agent(.finished, "Upgraded 12 packages", 120).text, "Finished · Upgraded 12 packages")
        XCTAssertEqual(agent(.failed, nil, 120).text, "Failed · 2m ago")
        XCTAssertEqual(agent(.failed, "rsync failed", 120).tone, .failed)
        XCTAssertEqual(agent(.idle, "ignored", 300).text, "Idle", "an idle agent without a folder says Idle")
    }

    func testAnIdleAgentSaysWhereItIs() {
        let idle = TerminalSummary(id: 1, title: "t", agent: .codex, folder: "/Users/me/Projects/app")
        XCTAssertEqual(line(idle, home: "/Users/me").text, "~/Projects/app")
    }

    func testTimesReadShort() {
        XCTAssertNil(SidebarWording.took(59))
        XCTAssertEqual(SidebarWording.took(180), "3m")
        XCTAssertEqual(SidebarWording.took(61 * 60), "1h")
        XCTAssertEqual(SidebarWording.took(49 * 3600), "2d")
        XCTAssertEqual(SidebarWording.ago(3600), "1h ago")
        XCTAssertEqual(SidebarWording.clock(5), "0:05")
        XCTAssertEqual(SidebarWording.clock(72), "1:12")
        XCTAssertEqual(SidebarWording.clock(3723), "1:02:03")
    }

    func testPathsGiveWayAtTheFront() {
        XCTAssertEqual(SidebarWording.pathTail("~/Projects/app", max: 20), "~/Projects/app")
        XCTAssertEqual(SidebarWording.pathTail("~/Projects/clients/acme/app", max: 18), "…/clients/acme/app")
        XCTAssertEqual(SidebarWording.displayPath("/Users/me", home: "/Users/me"), "~")
        XCTAssertEqual(SidebarWording.displayPath("/Users/mean/x", home: "/Users/me"), "/Users/mean/x")
    }

    func testAPlainTerminalSaysItsCommandWithTheClockAtTheRight() {
        let build = TerminalSummary(id: 1, title: "t", runningCommand: "cargo test", runningSince: ago(72))
        XCTAssertEqual(line(build), SecondLine(text: "cargo test", isCommand: true, trail: "1:12"))
        var unseen = build
        unseen.unread = 2
        XCTAssertEqual(line(unseen).trail, "new output")
    }

    func testAFailedCommandSaysHowItEnded() {
        let failed = TerminalSummary(
            id: 1, title: "t", lastFailure: CommandFailure(command: "pnpm deploy", exitCode: 1, at: ago(120)))
        XCTAssertEqual(line(failed), SecondLine(text: "pnpm deploy · exit 1", tone: .failed, isCommand: true, trail: "2m ago"))
        var bare = failed
        bare.lastFailure?.command = nil
        XCTAssertEqual(line(bare).text, "exit 1")
    }

    func testAQuietShellSaysWhereItIsOrThatItIsAShell() {
        XCTAssertEqual(line(TerminalSummary(id: 1, title: "t", folder: "~/infra")).text, "~/infra")
        XCTAssertEqual(line(TerminalSummary(id: 1, title: "t")).text, "Shell")
    }

    func testTheNextChangeIsTheRestOfTheUnit() {
        let working = TerminalSummary(id: 1, title: "t", activity: .working, agent: .claude, activitySince: ago(45))
        XCTAssertEqual(SidebarWording.nextChange(for: working, now: now), 15)
        var says = working
        says.message = "Editing…"
        XCTAssertNil(SidebarWording.nextChange(for: says, now: now))
        let running = TerminalSummary(id: 2, title: "t", runningCommand: "make", runningSince: ago(5.25))
        XCTAssertEqual(SidebarWording.nextChange(for: running, now: now) ?? 0, 0.75, accuracy: 0.001)
        XCTAssertNil(SidebarWording.nextChange(for: TerminalSummary(id: 3, title: "t"), now: now))
    }

    func testAgentNamesAreTheRegistrys() {
        XCTAssertEqual(AgentID.claude.shortName, "Claude")
        XCTAssertEqual(AgentID.codex.shortName, "Codex")
        XCTAssertEqual(AgentID.copilot.shortName, "GitHub Copilot")
        XCTAssertEqual(AgentID.fm.shortName, "Foundation Models")
    }
}
