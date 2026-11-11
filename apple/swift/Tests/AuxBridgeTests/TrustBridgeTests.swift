import AuxModel
import PlatformModel
import XCTest

@testable import AuxBridge

@MainActor
final class TrustBridgeTests: XCTestCase {
    private var scratch: Scratch!
    private var project: URL!

    override func setUpWithError() throws {
        scratch = try Scratch()
        project = try scratch.folder("api")
        try scratch.write(#"{"shell": "/bin/zsh", "env": {"FOO": "bar"}}"#, to: "api/.wormhole/config.json")
    }

    override func tearDown() async throws { scratch.remove() }

    private func backend(_ answer: ScriptedAuthenticator.Answer) throws -> FFITrustBackend {
        try FFITrustBackend(root: project, dataDir: scratch.data, authenticator: ScriptedAuthenticator(answer: answer))
    }

    func testTheProjectsLinesAreListedInFullBeforeAnyApproval() throws {
        let review = try backend(.cancel).review()
        XCTAssertEqual(review.name, "api")
        XCTAssertEqual(review.directory, ".wormhole")
        XCTAssertEqual(review.status, .untrusted)
        XCTAssertTrue(review.canApprove)
        XCTAssertTrue(review.lines.contains(TrustLine(what: "shell:", runs: "/bin/zsh")))
        XCTAssertTrue(review.lines.contains(TrustLine(what: "env:", runs: "FOO=bar")))
    }

    func testApprovingAsksTheOwnerFirstAndTrustsWhatWasShown() async throws {
        let backend = try backend(.allow)
        _ = backend.review()
        await backend.approve()
        XCTAssertEqual(backend.review().status, .trusted)
        XCTAssertFalse(backend.review().canApprove)
        // A git pull changes what runs: it must be approved again.
        try scratch.write(#"{"shell": "/bin/zsh", "env": {"FOO": "bar", "EVIL": "1"}}"#, to: "api/.wormhole/config.json")
        XCTAssertEqual(backend.review().status, .changed)
    }

    func testACancelledOrRefusedAuthenticationTrustsNothing() async throws {
        for answer in [ScriptedAuthenticator.Answer.cancel, .deny] {
            let backend = try backend(answer)
            _ = backend.review()
            await backend.approve()
            XCTAssertEqual(backend.review().status, .untrusted, "\(answer)")
        }
    }

    /// The owner's answer approves what the window drew, never what a `git pull` wrote while the dialog was up,
    /// even when the window's watch reviewed the new content meanwhile.
    func testAFileChangedDuringTheDialogIsNotTrusted() async throws {
        let (scratch, project) = (scratch!, project!)
        let evil = #"{"shell": "/bin/zsh", "env": {"FOO": "bar", "EVIL": "1"}}"#
        var during: (@MainActor () -> Void)?
        let backend = try FFITrustBackend(
            root: project, dataDir: scratch.data,
            authenticator: MeddlingAuthenticator {
                try? scratch.write(evil, to: "api/.wormhole/config.json")
                await during?()
            })
        during = { _ = backend.review() }
        _ = backend.review()
        await backend.approve()
        let after = backend.review()
        XCTAssertEqual(after.status, .changed)
        XCTAssertTrue(after.lines.contains(TrustLine(what: "env:", runs: "EVIL=1")))
    }

    func testNothingIsApprovedThatWasNeverReviewed() async throws {
        let backend = try backend(.allow)
        await backend.approve()
        XCTAssertEqual(backend.review().status, .untrusted)
    }

    func testTheViewModelFollowsTheBackendThroughApproval() async throws {
        let model = TrustViewModel(backend: try backend(.allow))
        XCTAssertEqual(model.approveLabel, "Trust (approve with password)")
        model.approve()
        for _ in 0..<500 where model.review.status != .trusted { try await Task.sleep(for: .milliseconds(10)) }
        XCTAssertEqual(model.review.status, .trusted)
    }

    func testAProjectThatDoesNotLoadHasNothingToApprove() throws {
        try scratch.write("{ not json", to: "api/.wormhole/config.json")
        let review = try backend(.allow).review()
        XCTAssertNotNil(review.problem)
        XCTAssertFalse(review.canApprove)
    }
}

/// Says yes, after running what a test wants to happen while the dialog is up.
private struct MeddlingAuthenticator: Authenticating {
    let meanwhile: @Sendable @MainActor () async -> Void
    func canAuthenticate() -> Bool { true }
    func biometricsAvailable() -> Bool { false }
    func authenticate(reason: String) async throws { await meanwhile() }
}
