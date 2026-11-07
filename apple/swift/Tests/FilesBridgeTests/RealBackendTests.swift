import Foundation
import PanelModel
import PlatformModel
import WormholeBindings
import XCTest

@testable import FilesBridge

/// The window's side of the bridge, recording what it was asked.
final class Recorder: PanelEffectPerforming, @unchecked Sendable {
    private let lock = NSLock()
    private var performed: [PanelEffect] = []
    private var problems: [String] = []

    func perform(_ effects: [PanelEffect]) async { lock.withLock { performed += effects } }
    func problem(_ text: String) async { lock.withLock { problems.append(text) } }

    var effects: [PanelEffect] { lock.withLock { performed } }
    var told: [String] { lock.withLock { problems } }
}

/// The backends against the real Rust core: real folders, a real database, a data folder of this run's own, and an
/// owner who answers from a script (a test never raises Touch ID).
final class RealBackendTests: XCTestCase {
    private var temp: URL!

    /// The library refuses to run on anything but a folder named Wormhole-swift*.
    private static let dataFolder = NSTemporaryDirectory() + "Wormhole-swift-files-test-\(getpid())"

    override class func setUp() { setenv("WORMHOLE_DATA_DIR", dataFolder, 1) }

    override class func tearDown() { try? FileManager.default.removeItem(atPath: dataFolder) }

    override func setUpWithError() throws {
        temp = FileManager.default.temporaryDirectory.appendingPathComponent("wh-files-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: temp, withIntermediateDirectories: true)
    }

    override func tearDownWithError() throws { try? FileManager.default.removeItem(at: temp) }

    private func backends(_ answer: ScriptedAuthenticator.Answer = .cancel) throws -> (PanelBackends, Recorder) {
        let recorder = Recorder()
        return (try FilesBridge.makeBackends(authenticator: ScriptedAuthenticator(answer: answer), effects: recorder), recorder)
    }

    private func write(_ text: String, _ path: String) throws {
        let url = temp.appendingPathComponent(path)
        try FileManager.default.createDirectory(at: url.deletingLastPathComponent(), withIntermediateDirectories: true)
        try text.write(to: url, atomically: true, encoding: .utf8)
    }

    private func firstWake(of stream: AsyncStream<Void>, within seconds: Double) async -> Bool {
        await withTaskGroup(of: Bool.self) { group in
            group.addTask { for await _ in stream { return true }; return false }
            group.addTask { try? await Task.sleep(for: .seconds(seconds)); return false }
            let first = await group.next() ?? false
            group.cancelAll()
            return first
        }
    }

    func testTheFilesTabListsReadsAndRenamesARealFolderAndRefusesALinkOut() async throws {
        let (backends, _) = try backends()
        let files = backends.files
        let outside = temp.appendingPathComponent("outside")
        try write("secret", "outside/secret.txt")
        try write("fn main() {}", "root/src/main.rs")
        try write("x", "root/b.txt")
        try FileManager.default.createSymbolicLink(at: temp.appendingPathComponent("root/jump"), withDestinationURL: outside)
        let root = temp.appendingPathComponent("root").path

        guard case let .success(listed) = await files.list("", in: root) else { return XCTFail("lists") }
        XCTAssertEqual(listed.entries.map(\.name), ["src", "b.txt", "jump"])
        XCTAssertEqual(listed.entries.map(\.kind), [.folder, .file, .link])
        if case .success = await files.list("jump", in: root) { XCTFail("a link is not a folder to enter") }
        guard case .failure(.gone) = await files.list("nothing", in: root) else { return XCTFail("gone") }

        let content = await files.read("src/main.rs", in: root)
        XCTAssertEqual(content, .text("fn main() {}", size: 12, truncated: false))
        let leak = await files.read("jump/secret.txt", in: root)
        XCTAssertEqual(leak, .unavailable(.outside))

        guard case .success = await files.rename("b.txt", to: "c.txt", in: root) else { return XCTFail("renames") }
        guard case .failure(.exists) = await files.rename("c.txt", to: "jump", in: root) else { return XCTFail("exists") }
        guard case .failure(.name) = await files.rename("c.txt", to: "a/b", in: root) else { return XCTFail("name") }
        guard case .failure(.gone) = await files.rename("jump/secret.txt", to: "mine", in: root) else { return XCTFail("link") }
        XCTAssertTrue(FileManager.default.fileExists(atPath: outside.appendingPathComponent("secret.txt").path))
    }

    func testAnUntrustedProjectRefusesAndATrustedOneHandsItsEffectsToTheWindow() async throws {
        let (backends, recorder) = try backends(.allow)
        let project = backends.project
        try write(#"[{"id": "t", "title": "Test", "type": "run", "command": "make test"}]"#, "repo/.wormhole/actions.json")
        try FileManager.default.createDirectory(at: temp.appendingPathComponent("repo/.git"), withIntermediateDirectories: true)
        let folder = temp.appendingPathComponent("repo").resolvingSymlinksInPath().path

        let beforeFound = await project.project(at: folder)
        let before = try XCTUnwrap(beforeFound)
        XCTAssertEqual(before.status, .untrusted)
        XCTAssertTrue(before.actions.isEmpty, "held back until trusted")
        await project.run(action: "t", at: before.root)
        XCTAssertEqual(recorder.effects, [], "nothing reaches the window")
        XCTAssertEqual(recorder.told.count, 1, "and the refusal is said: \(recorder.told)")

        let approved = await project.approve(at: before.root)
        XCTAssertTrue(approved)
        let afterFound = await project.project(at: folder)
        let after = try XCTUnwrap(afterFound)
        XCTAssertEqual(after.status, .trusted)
        XCTAssertEqual(after.actions.map(\.title), ["Test"])
        await project.run(action: "t", at: after.root)
        XCTAssertEqual(recorder.effects, [.type("make test", in: .current)])

        await project.revoke(at: after.root)
        await project.run(action: "t", at: after.root)
        XCTAssertEqual(recorder.effects.count, 1, "revoked: nothing more runs")
    }

    func testADeclinedApprovalTrustsNothing() async throws {
        let (backends, _) = try backends(.cancel)
        try write(#"[{"id": "t", "title": "Test", "type": "run", "command": "make"}]"#, "repo/.wormhole/actions.json")
        try FileManager.default.createDirectory(at: temp.appendingPathComponent("repo/.git"), withIntermediateDirectories: true)
        let folder = temp.appendingPathComponent("repo").resolvingSymlinksInPath().path
        let stateFound = await backends.project.project(at: folder)
        let state = try XCTUnwrap(stateFound)
        let approved = await backends.project.approve(at: state.root)
        XCTAssertFalse(approved)
        let still = await backends.project.project(at: folder)
        XCTAssertEqual(still?.status, .untrusted)
    }

    func testTheProjectWatchWakesWhenTheFolderChanges() async throws {
        let (backends, _) = try backends()
        try write("{}", "repo/.wormhole/config.json")
        try FileManager.default.createDirectory(at: temp.appendingPathComponent("repo/.git"), withIntermediateDirectories: true)
        let folder = temp.appendingPathComponent("repo").resolvingSymlinksInPath().path
        let stream = backends.project.changes(at: folder)
        try await Task.sleep(for: .milliseconds(500))
        try write("{\"font_size\": 14}", "repo/.wormhole/config.json")
        let woke = await firstWake(of: stream, within: 10)
        XCTAssertTrue(woke)
    }

    func testUsageReportsAreKeptAndWakeTheTab() async throws {
        let (backends, _) = try backends()
        let usage = backends.usage
        let stream = usage.changes()
        let json = #"{"rate_limits": {"five_hour": {"used_percentage": 25.0, "resets_at": 1800002040}}}"#
        XCTAssertTrue(usage.recordStatusLine(json, now: Date(timeIntervalSince1970: 1_800_000_000)))
        let woke = await firstWake(of: stream, within: 5)
        XCTAssertTrue(woke)
        let reports = await usage.reports()
        XCTAssertEqual(reports.first?.agent, "claude")
        XCTAssertEqual(reports.first?.limits, [PlanLimit(label: "5-hour limit", used: 25, resetsAt: 1_800_002_040)])
        XCTAssertFalse(usage.recordStatusLine("{}"), "no limits, no report")
    }

    func testNotesComeBackAsSaved() throws {
        let (backends, _) = try backends()
        var model = NotesModel(notes: [Note(text: "a note", collapsed: true), Note(text: "")], tasks: TaskList([PanelTask(text: "step", done: true)]))
        model.stepsCollapsed = true
        XCTAssertTrue(backends.notes.save(session: 4242, model: model))
        let back = backends.notes.load(session: 4242)
        XCTAssertEqual(back.notes.map(\.text), ["a note"], "a blank note is never kept")
        XCTAssertEqual(back.notes.map(\.collapsed), [true])
        XCTAssertEqual(back.tasks.items.map(\.text), ["step"])
        XCTAssertEqual(backends.notes.load(session: 4243), NotesModel())
    }
}
