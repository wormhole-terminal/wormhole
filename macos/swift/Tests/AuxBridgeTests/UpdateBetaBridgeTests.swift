import AuxModel
import XCTest
import WormholeBindings

@testable import AuxBridge

/// The beta build's updater over core, against a loopback feed that offers more on Stable's and Preview's than on its own.
@MainActor
final class UpdateBetaBridgeTests: XCTestCase {
    private var scratch: Scratch!
    private var feed: LoopbackFeed?

    override func setUpWithError() throws { scratch = try Scratch() }

    override func tearDown() async throws {
        feed?.stop()
        scratch.remove()
    }

    /// A `latest.json` offering `version` for both Mac architectures. The archive is never asked for; the signature has
    /// minisign's shape (the feed is refused without one) and signs nothing.
    private func release(_ version: String) -> String {
        let signature = [
            "untrusted comment: a test's", Data([0x45, 0x64] + [UInt8](repeating: 0, count: 72)).base64EncodedString(),
            "trusted comment: a test's", Data(repeating: 0, count: 64).base64EncodedString(),
        ].joined(separator: "\n")
        let asset: [String: Any] = ["url": "http://127.0.0.1:1/never-asked.tar.gz", "sha256": String(repeating: "ab", count: 32), "signature": signature, "size": 1234]
        let json: [String: Any] = ["version": version, "notes": "", "platforms": ["darwin-aarch64": asset, "darwin-x86_64": asset]]
        return String(decoding: (try? JSONSerialization.data(withJSONObject: json)) ?? Data(), as: UTF8.self)
    }

    private func beta(current: String) async throws -> FFIUpdateBackend {
        let server = try LoopbackFeed(routes: [
            "/beta/latest.json": release("2.0.0-beta.2"), "/stable/latest.json": release("9.0.0"), "/unstable/latest.json": release("9.0.0-dev.1"),
        ])
        feed = server
        var setup = FFIUpdateBackend.Setup(baseURL: try await server.start(), publicKey: "", currentVersion: current, bundleID: "dev.wormhole.test")
        setup.scheduleFile = scratch.root.appendingPathComponent("schedule.json")
        setup.channel = .beta
        let backend = try FFIUpdateBackend(setup: setup, hooks: .init(terminate: { XCTFail("nothing here quits the app") }))
        // On by default, a check would go on to install what it finds; these ask only what is offered.
        backend.apply(autoUpdate: false)
        return backend
    }

    private func offered(_ backend: FFIUpdateBackend) -> String? {
        if case .available(let release) = backend.phase { return release.version }
        return nil
    }

    func testABetaIsOfferedTheNewerBetaFromItsOwnFeedOnly() async throws {
        let backend = try await beta(current: "2.0.0-beta.1")
        XCTAssertEqual(backend.channel, .beta)
        await backend.check()
        XCTAssertEqual(offered(backend), "2.0.0-beta.2", "not Stable's 9.0.0 nor Preview's 9.0.0-dev.1")
    }

    func testAnAppThatRunsAsItsShortVersionIsNeverOfferedAnotherBeta() async throws {
        // Why the app runs as `WormholeVersion`: as `CFBundleShortVersionString` it is 2.0.0, above every beta of 2.0.0.
        let backend = try await beta(current: "2.0.0")
        await backend.check()
        XCTAssertEqual(backend.phase, .upToDate)
    }

    func testNothingMovesABetaOffItsFeed() async throws {
        let backend = try await beta(current: "2.0.0-beta.1")
        backend.channel = .preview
        backend.channel = .stable
        XCTAssertEqual(backend.channel, .beta, "the Settings value does not reach it")
        await backend.check()
        XCTAssertEqual(offered(backend), "2.0.0-beta.2")
    }

    func testTheBetaChannelCrossesTheBridgeBothWays() {
        XCTAssertEqual(UpdateChannel.beta.record, .beta)
        XCTAssertEqual(UpdateChannelRecord.beta.value, .beta)
    }
}
