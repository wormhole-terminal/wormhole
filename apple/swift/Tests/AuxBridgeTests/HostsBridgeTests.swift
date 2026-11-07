import AuxModel
import XCTest
import WormholeBindings

@testable import AuxBridge

@MainActor
final class HostsBridgeTests: XCTestCase {
    private var scratch: Scratch!

    override func setUpWithError() throws { scratch = try Scratch() }
    override func tearDown() async throws { scratch.remove() }

    /// A host's meta file whose process is long gone: its files are left over, which core calls stale.
    private func leftOverHost(_ id: String) throws -> URL {
        let hosts = try scratch.folder("Wormhole-swift-t/hosts")
        let text = "format=1\nid=\(id)\npid=2147483000\nstarted_at=1700000000\nhost_version=1.6.2\nproto_min=1\nproto_max=1\nsocket=\(hosts.path)/\(id).sock\ndata_dir=\(scratch.data.path)\n"
        let meta = hosts.appendingPathComponent("\(id).meta")
        try text.write(to: meta, atomically: true, encoding: .utf8)
        return meta
    }

    func testAFolderOutsideTheExperimentIsRefused() {
        XCTAssertThrowsError(try FFIHostsBackend(dataDir: URL(fileURLWithPath: "/tmp/Wormhole-dev")))
    }

    func testAnEmptyFolderHasNoCardsAndAStaleHostCanBeForgotten() async throws {
        let backend = try FFIHostsBackend(dataDir: scratch.data)
        backend.setPersistentTerminals(true)
        var report = await backend.report()
        XCTAssertTrue(report.cards.isEmpty)
        XCTAssertTrue(report.flagOn)
        XCTAssertTrue(report.folder.hasSuffix("Wormhole-swift-t/hosts"), report.folder)
        XCTAssertTrue(report.protocolText.contains("1"), report.protocolText)

        let meta = try leftOverHost("stalehost001")
        report = await backend.report()
        let card = try XCTUnwrap(report.cards.first)
        XCTAssertEqual(card.id, "stalehost001")
        XCTAssertEqual(card.standing, .stale)
        XCTAssertTrue(card.canForget && !card.canEnd)
        XCTAssertEqual(card.facts.first { $0.field == .hostPid }?.value, "2147483000")
        XCTAssertTrue(card.text.contains("stalehost001"))

        await backend.forget("stalehost001")
        report = await backend.report()
        XCTAssertTrue(report.cards.isEmpty)
        XCTAssertFalse(FileManager.default.fileExists(atPath: meta.path))
    }

    func testTheFolderWatchWakesTheWindowWhenAHostsFilesChange() async throws {
        let backend = try FFIHostsBackend(dataDir: scratch.data)
        _ = await backend.report()
        let changes = backend.changes()
        let woke = Task { @MainActor in for await _ in changes { return true }; return false }
        _ = try leftOverHost("stalehost002")
        let deadline = Task { try await Task.sleep(for: .seconds(15)); woke.cancel() }
        let result = await woke.value
        deadline.cancel()
        XCTAssertTrue(result, "no event came for a new meta file")
    }

    func testCardsAndFactsKeepTheirToneAndOrder() {
        let record = HostCardRecord(
            id: "abc", standing: .attached, note: "Window 1", facts: [
                HostFactRecord(field: .version, value: "1.6.2 (this app is 1.7.0)", tone: .warning),
                HostFactRecord(field: .socket, value: "/x.sock", tone: .muted),
            ], attachedTitle: "api", needsAttention: true)
        let card = record.value
        XCTAssertEqual(card.standing, .attached)
        XCTAssertEqual(card.attachedTitle, "api")
        XCTAssertTrue(card.needsAttention)
        XCTAssertEqual(card.facts.map(\.field), [.version, .socket])
        XCTAssertEqual(card.facts.map(\.tone), [.warning, .muted])
        let report = HostsReportRecord(appVersion: "1.7.0", protocolText: "1 (minor 4)", folder: "/h", flagOn: false, program: .disposableCopy, programPath: nil, cards: [record]).value
        XCTAssertEqual(report.program, .disposableCopy)
        XCTAssertEqual(report.count(.attached), 1)
    }
}
