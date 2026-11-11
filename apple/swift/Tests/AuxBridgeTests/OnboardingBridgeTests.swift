import AuxModel
import PlatformModel
import XCTest

@testable import AuxBridge

@MainActor
final class OnboardingBridgeTests: XCTestCase {
    private var scratch: Scratch!
    private static let agentPath: URL = {
        // Agents are looked for here and nowhere else, so what the Mac has installed never shows. Before any call.
        let folder = FileManager.default.temporaryDirectory.appendingPathComponent("aux-bridge-agents-\(UUID().uuidString.prefix(8))")
        try? FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true)
        let fake = folder.appendingPathComponent("claude")
        try? "#!/bin/sh\n".write(to: fake, atomically: true, encoding: .utf8)
        try? FileManager.default.setAttributes([.posixPermissions: 0o755], ofItemAtPath: fake.path)
        setenv("WORMHOLE_AGENT_PATH", folder.path, 1)
        return folder
    }()

    override func setUpWithError() throws {
        _ = Self.agentPath
        scratch = try Scratch()
    }

    override func tearDown() async throws { scratch.remove() }

    private var finished: (URL?, Bool)?

    private func backend(project: URL? = nil) throws -> FFIOnboardingBackend {
        try FFIOnboardingBackend(
            preferences: OnboardingPreferences(), settingsKeys: "\u{2318},", project: project,
            recents: { [self] in [scratch.root] }, agentHome: scratch.root.appendingPathComponent("agents"), dataDir: scratch.data,
            authenticator: ScriptedAuthenticator(answer: .allow), onFinish: { [self] in finished = ($0, $1) })
    }

    func testTheAgentsOnPathAreListedAndOnlyThePressedOnesHooksAreWritten() async throws {
        let backend = try backend()
        let scan = await backend.scanAgents()
        XCTAssertEqual(scan.found.map(\.id), ["claude"])
        XCTAssertEqual(scan.found.first?.hooks, .notInstalled)
        XCTAssertGreaterThan(scan.others, 0)
        XCTAssertFalse(FileManager.default.fileExists(atPath: scratch.root.appendingPathComponent("agents").path), "scanning writes nothing")

        let after = try await backend.installHooks(agent: "claude")
        XCTAssertEqual(after.hooks, .installed)
        XCTAssertTrue(FileManager.default.fileExists(atPath: scratch.root.appendingPathComponent("agents/.claude/settings.json").path))
        do {
            _ = try await backend.installHooks(agent: "nobody")
            XCTFail("an unknown agent has no hooks")
        } catch {}
    }

    func testTheFirstStepOffersRecentFoldersFirst() throws {
        let places = try backend().places()
        XCTAssertEqual(places.first?.kind, .recent)
        XCTAssertEqual(places.first?.url.standardizedFileURL.path, scratch.root.standardizedFileURL.path)
        XCTAssertTrue(places.contains { $0.kind == .home })
    }

    func testFinishingRemembersAFirstRunAndALeavesAReplayAlone() throws {
        let backend = try backend()
        XCTAssertTrue(backend.shouldShow(savedWindows: 0))
        backend.finish(folder: nil, firstRun: false)
        XCTAssertTrue(backend.shouldShow(savedWindows: 0), "a replay changes nothing")
        XCTAssertEqual(finished?.1, false)
        backend.finish(folder: scratch.root, firstRun: true)
        XCTAssertFalse(backend.shouldShow(savedWindows: 0))
        XCTAssertEqual(finished?.0, scratch.root)
    }

    func testTheLookStepWritesThroughCoreSettingsAsThePersonGoes() throws {
        let backend = try backend()
        backend.preferences.fontSize = 15
        backend.preferences.ligatures = false
        let text = try String(contentsOf: scratch.data.appendingPathComponent("config.json"), encoding: .utf8)
        let json = try XCTUnwrap(try JSONSerialization.jsonObject(with: Data(text.utf8)) as? [String: Any])
        XCTAssertEqual(json["font_size"] as? Double, 15)
        XCTAssertEqual(json["font_ligatures"] as? Bool, false)
    }

    func testAProjectToTrustGetsACardAndATrustedOneDoesNot() async throws {
        let project = try scratch.folder("api")
        try scratch.write(#"{"shell": "/bin/zsh", "env": {"A": "1"}}"#, to: "api/.wormhole/config.json")
        let backend = try backend(project: project)
        XCTAssertEqual(backend.trustCard, TrustCard(name: "api", directory: ".wormhole", commands: 2))
        XCTAssertNil(try self.backend().trustCard, "no project")
    }
}
