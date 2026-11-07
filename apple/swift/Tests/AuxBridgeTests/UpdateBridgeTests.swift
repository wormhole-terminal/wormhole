import AuxModel
import XCTest
import WormholeBindings

@testable import AuxBridge

@MainActor
final class UpdateBridgeTests: XCTestCase {
    private var scratch: Scratch!
    private var feed: LoopbackFeed?

    override func setUpWithError() throws { scratch = try Scratch() }

    override func tearDown() async throws {
        feed?.stop()
        scratch.remove()
    }

    private func backend(feed routes: [String: String]) async throws -> FFIUpdateBackend {
        let server = try LoopbackFeed(routes: routes)
        feed = server
        var setup = FFIUpdateBackend.Setup(baseURL: try await server.start(), publicKey: "", currentVersion: "1.0.0", bundleID: "dev.wormhole.test")
        setup.scheduleFile = scratch.root.appendingPathComponent("schedule.json")
        return try FFIUpdateBackend(setup: setup, hooks: .init(terminate: { XCTFail("nothing here quits the app") }))
    }

    func testAFeedWithNothingNewIsUpToDateAndRemembersWhenItLooked() async throws {
        let backend = try await backend(feed: ["/stable/latest.json": #"{"version":"0.9.0","platforms":{}}"#])
        XCTAssertEqual(backend.phase, .idle)
        XCTAssertNil(backend.lastChecked)
        await backend.check()
        XCTAssertEqual(backend.phase, .upToDate)
        XCTAssertNotNil(backend.lastChecked)
        XCTAssertEqual(backend.channel, .stable)
    }

    func testAFeedThatDoesNotAnswerIsAFailureSaidToThePerson() async throws {
        let backend = try await backend(feed: [:])
        await backend.check()
        guard case .failed(let message) = backend.phase else { return XCTFail("\(backend.phase)") }
        XCTAssertTrue(message.contains("network error"), message)
    }

    func testPhasesStartWithTheCurrentOneAndFollowTheChecks() async throws {
        let backend = try await backend(feed: ["/stable/latest.json": #"{"version":"0.9.0","platforms":{}}"#])
        var seen: [UpdatePhase] = []
        let watcher = Task { @MainActor in
            for await phase in backend.phases() {
                seen.append(phase)
                if phase == .upToDate { return }
            }
        }
        Task { await backend.check() }
        await watcher.value
        XCTAssertEqual(seen.first, .idle)
        XCTAssertEqual(seen.last, .upToDate)
    }

    func testTheChannelGoesToCoreAndBack() async throws {
        let backend = try await backend(feed: ["/stable/latest.json": #"{"version":"0.9.0","platforms":{}}"#])
        backend.channel = .preview
        XCTAssertEqual(backend.channel, .preview)
        backend.channel = .stable
        XCTAssertEqual(backend.channel, .stable)
    }

    func testRestartWithoutABundleChangesNothingAndDoesNotQuit() async throws {
        let backend = try await backend(feed: [:])
        backend.restart(whenIdle: false)
        XCTAssertEqual(backend.phase, .failed("Wormhole isn't running from an app bundle."))
    }

    func testTheEnginesPhasesBecomeTheWindowsValues() {
        let release = UpdateReleaseRecord(version: "1.7.0", notes: "## Fixed", published: "2026-10-01T12:00:00Z", isSecurity: true)
        XCTAssertEqual(
            UpdatePhaseRecord.available(release: release).value,
            .available(Release(version: "1.7.0", notes: "## Fixed", published: try? Date("2026-10-01T12:00:00Z", strategy: .iso8601), isSecurity: true)))
        XCTAssertEqual(UpdatePhaseRecord.downloading(received: 5, expected: 20).value, .downloading(received: 5, expected: 20))
        XCTAssertEqual(UpdatePhaseRecord.installed(version: "1.7.0").value, .installed(version: "1.7.0"))
        XCTAssertEqual(UpdatePhaseRecord.unsupported(reason: "needs macOS 27").value, .unsupported("needs macOS 27"))
        XCTAssertEqual(UpdateChannel.preview.record, .preview)
        XCTAssertEqual(UpdateChannelRecord.stable.value, .stable)
    }
}
