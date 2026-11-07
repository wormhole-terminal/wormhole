import AppKit
import XCTest
@testable import AuxModel
@testable import AuxWindows

/// The questions' words and rules, ported from the GPUI app's own tests (`window::quit_question`, `session`,
/// `update::restart`), and the alerts built from them. The alert is the system's window and is not drawn here.
@MainActor
final class QuestionTests: XCTestCase {
    private func busy(_ command: Bool = true) -> StillRunning.Fact { .init(status: .busy, commandRunning: command) }

    func testQuittingSaysTheTerminalsComeBackButTheirProgramsEnd() {
        XCTAssertEqual(
            QuitQuestion.detail(running: [.init(status: .idle, commandRunning: true)], programs: ["claude", "caffeinate"]),
            "Your terminals come back next time, but what runs in them ends. Still running: claude, caffeinate.")
    }

    func testQuittingNamesTheOtherProfilesThatStillRunSomething() {
        XCTAssertEqual(
            QuitQuestion.detail(running: [busy(false)], programs: [], otherProfiles: [("Work", 1), ("Client", 3)]),
            "Your terminals come back next time, but what runs in them ends. An agent is still working. "
                + "1 terminal in Work is running an agent or a command. "
                + "3 terminals in Client are running an agent or a command.")
    }

    func testQuittingWithTheBoxSaysWhatGoesOnAndWhatEnds() {
        let running = [StillRunning.Fact(status: .idle, commandRunning: true)]
        XCTAssertEqual(
            QuitQuestion.detail(running: running, programs: ["claude"], hostedRunning: 2, localRunning: 0),
            "Your terminals come back next time and what runs in them keeps going. Untick the box to end them. Still running: claude.")
        XCTAssertEqual(
            QuitQuestion.detail(running: running, programs: ["claude"], otherProfiles: [("Work", 1)], hostedRunning: 2, localRunning: 1),
            "Your terminals come back next time. 2 terminals keep running; 1 terminal ends because it runs inside Wormhole. "
                + "Untick the box to end the others too. Still running: claude. 1 terminal in Work is running an agent or a command.")
        XCTAssertTrue(
            QuitQuestion.detail(running: running, programs: [], hostedRunning: 1, localRunning: 3)
                .contains("1 terminal keeps running; 3 terminals end because they run inside Wormhole."))
    }

    func testTheBoxShowsWhenATerminalIsHostedAndSomethingRuns() {
        XCTAssertNil(QuitQuestion.ask(endHosts: false, hostedRunning: 0, localRunning: 0, anyHosted: true))
        XCTAssertEqual(QuitQuestion.ask(endHosts: false, hostedRunning: 0, localRunning: 2, anyHosted: false), .plain)
        XCTAssertEqual(QuitQuestion.ask(endHosts: false, hostedRunning: 1, localRunning: 2, anyHosted: true), .keepBox(localRunning: true))
        XCTAssertEqual(QuitQuestion.ask(endHosts: true, hostedRunning: 1, localRunning: 1, anyHosted: true), .plain)
        XCTAssertFalse(QuitQuestion.endsHosts(keepRunning: true))
        XCTAssertTrue(QuitQuestion.endsHosts(keepRunning: false))
    }

    func testQuitAndEndAllIsAPlainQuestionThatEndsTheHosts() throws {
        let facts = QuitFacts(running: [busy()], hostedRunning: 1, localRunning: 0, anyHosted: true)
        let end = try XCTUnwrap(facts.question(endAll: true))
        XCTAssertEqual(end.title, "Quit and End All Terminals?")
        XCTAssertEqual(end.confirm, "Quit and End All")
        XCTAssertNil(end.checkbox)
        XCTAssertTrue(QuitFacts.endsHosts(endAll: true, answer: .confirm(boxChecked: true)))
        let keep = try XCTUnwrap(facts.question(endAll: false))
        XCTAssertEqual(keep.checkbox?.label, "Keep terminals running in the background")
        XCTAssertFalse(QuitFacts.endsHosts(endAll: false, answer: .confirm(boxChecked: true)))
        XCTAssertTrue(QuitFacts.endsHosts(endAll: false, answer: .confirm(boxChecked: false)))
        XCTAssertNil(QuitFacts().question(endAll: false), "nothing runs: no question")
    }

    func testClosingATerminalNamesWhatEnds() {
        XCTAssertEqual(CloseQuestions.sessionTitle("api"), "Close \u{201C}api\u{201D}?")
        func detail(_ agent: String?, _ stakes: [String] = [], _ status: RunningTerminal.Status = .idle, _ command: Bool = false, _ programs: [String] = []) -> String {
            CloseQuestions.session(agent: agent, stakes: stakes, status: status, commandRunning: command, programs: programs)
        }
        XCTAssertEqual(detail("Claude", [], .busy, true), "Claude is still working. Closing ends the process.")
        XCTAssertEqual(detail(nil), "Its shell ends.")
        XCTAssertEqual(detail(nil, ["3 notes and tasks are discarded."]), "Its shell ends. Its 3 notes and tasks are discarded.")
        XCTAssertEqual(detail(nil, [], .idle, true), "A command is still running. Closing ends the process.")
        XCTAssertEqual(detail(nil, [], .idle, true, ["a", "b"]), "Still running: a, b. Closing ends them.")
    }

    func testWindowAndGroupQuestions() {
        XCTAssertEqual(CloseQuestions.window(terminals: 1, running: [], programs: [], stakes: []), "The terminal in this window will end.")
        XCTAssertEqual(CloseQuestions.window(terminals: 3, running: [busy()], programs: [], stakes: []), "The 3 terminals in this window will end. An agent is still working.")
        XCTAssertEqual(CloseQuestions.groupTitle("api"), "Close the group \u{201C}api\u{201D}?")
        XCTAssertEqual(CloseQuestions.group(terminals: 2, running: [], programs: [], stakes: ["notes go too."]), "The 2 terminals in this group will end. Their notes go too.")
    }

    func testTheProgramListIsCutShortAndEndsInAndNMore() {
        XCTAssertEqual(StillRunning.list(["a", "a", "b"]), "a, b")
        XCTAssertEqual(StillRunning.list(["1", "2", "3", "4", "5", "6"]), "1, 2, 3, 4 and 2 more")
        XCTAssertNil(StillRunning.list([]))
        XCTAssertEqual(StillRunning.shortened(String(repeating: "x", count: 50)), String(repeating: "x", count: 40) + "…")
    }

    func testTheRestartListsWhatItEndsAndOffersToWait() throws {
        XCTAssertNil(RestartQuestion.question([]), "nothing runs: it restarts at once")
        let running = [RunningTerminal(name: "api", agent: "Claude", status: .busy), RunningTerminal(name: "npm run dev", status: .idle, program: "npm run dev")]
        let question = try XCTUnwrap(RestartQuestion.question(running))
        XCTAssertEqual(question.detail, "api \u{2014} Claude is working\nnpm run dev\n\nYour terminals come back after the restart, but what runs in them ends.")
        XCTAssertEqual(question.alsoButtons, ["Restart When Idle"])
        XCTAssertEqual(question.cancel, "Later")
        let many = (1...9).map { RunningTerminal(name: "t\($0)", status: .busy) }
        XCTAssertTrue(RestartQuestion.detail(many).contains("and 3 more"))
    }

    func testEndingAHostNamesTheTerminal() {
        XCTAssertEqual(CloseQuestions.endHost(id: "h-1", attachedTitle: "api").detail, "The program in the terminal \u{201C}api\u{201D} gets a hangup and the terminal ends.")
        XCTAssertEqual(CloseQuestions.endHost(id: "h-1", attachedTitle: nil).detail, "The program in it gets a hangup and ends.")
        XCTAssertEqual(CloseQuestions.unavailableHost(name: "api").alsoButtons, ["Leave It Running"])
    }

    func testTheAlertHasTheDefaultFirstAndCancelLast() {
        let sheet = QuestionSheet(RestartQuestion.question([RunningTerminal(name: "a", status: .busy)])!)
        XCTAssertEqual(sheet.buttonTitles, ["Restart", "Restart When Idle", "Later"])
        let quit = QuestionSheet(QuitFacts(running: [busy()], hostedRunning: 1, anyHosted: true).question(endAll: false)!)
        XCTAssertEqual(quit.buttonTitles, ["Quit", "Cancel"])
    }
}
