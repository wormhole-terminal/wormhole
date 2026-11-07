import AuxModel
import XCTest
import WormholeBindings

@testable import AuxBridge

@MainActor
final class AccessBridgeTests: XCTestCase {
    private var scratch: Scratch!

    override func setUpWithError() throws { scratch = try Scratch() }
    override func tearDown() async throws { scratch.remove() }

    private func question(about terminal: Int64? = nil) -> AccessQuestionRecord {
        AccessQuestionRecord(
            id: 7, label: "Claude Code", capability: .run, kind: .developerId,
            program: AccessProgramRecord(path: "~/.local/bin/claude", pid: 4182, signer: .developerId, subject: "Anthropic PBC", team: "Q6L2SF6YDW", interpreter: nil, startedBy: "zsh"),
            aboutTerminal: terminal, unusualSigner: false, changedSinceAllowed: true, elevated: false, canBeRemembered: true)
    }

    func testAnEmptyDeskHasNoQuestionAndAnswersToNothingAreIgnored() throws {
        let backend = try FFIAccessBackend(dataDir: scratch.data)
        XCTAssertNil(backend.next())
        backend.answer(99, .deny)
        backend.answer(99, .keep)
        backend.withdraw(99)
        XCTAssertNil(backend.next())
    }

    func testAQuestionBecomesTheSheetsRequestWithTheWindowsFacts() throws {
        let resolver = FFIAccessBackend.Resolver(terminalOf: { $0 == 4182 ? "api" : nil })
        let backend = try FFIAccessBackend(dataDir: scratch.data, resolver: resolver)
        let request = backend.request(of: question())
        XCTAssertEqual(request.kind, .developerID)
        XCTAssertEqual(request.program?.runsIn, "api")
        XCTAssertEqual(request.program?.signer, .init(kind: .developerID, subject: "Anthropic PBC", team: "Q6L2SF6YDW"))
        XCTAssertTrue(request.changedSinceAllowed && request.canBeRemembered)
        let display = AccessDisplay.of(request)
        XCTAssertEqual(display.title, "Allow \u{201C}Claude Code\u{201D} to control Wormhole?")
        XCTAssertTrue(display.detail.contains("changed since you allowed it."))
        XCTAssertTrue(display.accessory.contains("Anthropic PBC (Developer ID, team Q6L2SF6YDW)"))
    }

    func testAQuestionAboutATerminalNamesItAndFallsBackToItsNumber() throws {
        let about = question(about: 12)
        let named = try FFIAccessBackend(dataDir: scratch.data, resolver: .init(terminal: { _ in .init(name: "api", folder: "~/Projects/api") }))
        XCTAssertEqual(named.request(of: about).terminal, .init(name: "api", folder: "~/Projects/api"))
        let unnamed = try FFIAccessBackend(dataDir: scratch.data)
        XCTAssertEqual(unnamed.request(of: about).terminal, .init(name: "12"))
    }

    func testAFolderOutsideTheExperimentIsRefused() {
        XCTAssertThrowsError(try FFIAccessBackend(dataDir: URL(fileURLWithPath: "/Users/someone/Library/Application Support/Wormhole")))
    }
}
