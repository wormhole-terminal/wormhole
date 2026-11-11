import XCTest
@testable import AuxModel
@testable import AuxWindows

@MainActor
final class AccessTests: XCTestCase {
    private func request(_ kind: AccessRequest.Kind = .developerID, _ capability: AccessRequest.Capability = .run) -> AccessRequest {
        AccessRequest(
            label: "Claude Code", capability: capability, kind: kind,
            program: .init(
                path: "~/.local/bin/claude", pid: 4182, signer: .init(kind: .developerID, subject: "Anthropic PBC", team: "Q6L2SF6YDW"),
                startedBy: "zsh", runsIn: "api"),
            canBeRemembered: true)
    }

    func testAProgramGetsTheFourButtonsAndItsFacts() {
        let display = AccessDisplay.of(request())
        XCTAssertEqual(display.title, "Allow \u{201C}Claude Code\u{201D} to control Wormhole?")
        XCTAssertEqual(display.buttons.map(\.label), ["Don\u{2019}t Allow", "Allow for This Session", "Until Wormhole Quits", "Always"])
        XCTAssertTrue(display.detail.hasPrefix("It wants to run commands in terminals.\n\nAllowing lets this program"))
        XCTAssertEqual(
            display.accessory,
            "Path        ~/.local/bin/claude\nPID         4182\nSigned by   Anthropic PBC (Developer ID, team Q6L2SF6YDW)\nStarted by  zsh\nRuns in     terminal \u{201C}api\u{201D}")
    }

    func testScriptsAreAskedAboutTogetherAndNeverGetAlways() {
        var scripts = request(.scripts)
        scripts.canBeRemembered = false
        let display = AccessDisplay.of(scripts)
        XCTAssertEqual(display.title, "Allow scripts to control Wormhole?")
        XCTAssertFalse(display.buttons.contains(.always))
        XCTAssertTrue(display.accessory.hasPrefix("Asked by"))
    }

    func testATerminalQuestionHasNoAlwaysAndNamesTheTerminal() {
        var about = request()
        about.terminal = .init(name: "api", folder: "~/Projects/api")
        let display = AccessDisplay.of(about)
        XCTAssertEqual(display.title, "Allow \u{201C}Claude Code\u{201D} to run commands in \u{201C}api\u{201D}?")
        XCTAssertEqual(display.buttons.map(\.label), ["Don\u{2019}t Allow", "Allow for This Session", "Until Wormhole Quits"])
        XCTAssertEqual(display.accessory, "Terminal  api\nFolder    ~/Projects/api")
    }

    func testWarningsFollowTheFacts() {
        var risky = request(.adHoc)
        (risky.unusualSigner, risky.changedSinceAllowed, risky.elevated) = (true, true, true)
        let detail = AccessDisplay.of(risky).detail
        for line in ["is not signed by its usual developer.", "ad hoc signature", "changed since you allowed it.", "runs as root."] {
            XCTAssertTrue(detail.contains(line), line)
        }
    }

    func testAnEditedModeAsksToKeepItAndDefaultsToPuttingTheOldOneBack() {
        let display = AccessDisplay.modeChanged(previous: "Ask", requested: "Open")
        XCTAssertEqual(display.buttons.map(\.label), ["Don\u{2019}t Keep", "Keep"])
        XCTAssertTrue(display.title.contains("\u{201C}Open\u{201D}"))
    }

    func testTheSheetDefaultsToTheButtonThatBacksOut() {
        XCTAssertEqual(AccessSheet(request()).buttonTitles.first, "Don\u{2019}t Allow")
    }
}
