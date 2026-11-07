// The hook socket and the control line against the real library: a command line as an agent's shell runs it, an
// agent's hook for a terminal of the workspace, and what the app is left to do.

import Foundation
import XCTest
import WormholeBindings

@testable import HookBridge

private final class Quiet: WorkspaceListener, PanelListener, AuxListener, TerminalListener, @unchecked Sendable {
    func wake() {}
}

@MainActor
final class HookBridgeTests: XCTestCase {
    private var root: URL!
    private var data: URL!
    private let quiet = Quiet()

    override func setUpWithError() throws {
        root = FileManager.default.temporaryDirectory.appendingPathComponent("hook-bridge-\(UUID().uuidString.prefix(8))")
        data = root.appendingPathComponent("Wormhole-swift-t")
        try FileManager.default.createDirectory(at: data, withIntermediateDirectories: true)
        let config = #"{"agent_access":"open","control_write":true,"feature_flags":{"automation":true}}"#
        try config.write(to: data.appendingPathComponent("config.json"), atomically: true, encoding: .utf8)
        // The library refuses to touch anything but the experiment's own folder.
        setenv("WORMHOLE_DATA_DIR", data.path, 1)
    }

    override func tearDown() async throws { try? FileManager.default.removeItem(at: root) }

    private func makeServer() throws -> (Workspace, HookServerHandle) {
        let workspace = Workspace(
            config: WorkspaceConfig(home: nil, numbersFollowList: true, startFolder: nil, paneHeaderHeight: 0),
            listener: quiet)
        let usage = try UsageService(listener: quiet)
        let desk = try AccessDesk(profile: nil, dataDir: data.path, listener: quiet)
        return (workspace, try HookBridge.start(workspace: workspace, usage: usage, access: desk, dataDir: data))
    }

    /// A terminal running a script named like an agent (`claude`, `codex`), which the registry takes for the agent it names.
    private func agentTerminal(_ workspace: Workspace, named name: String = "claude") throws -> Int64 {
        let script = root.appendingPathComponent(name)
        try "#!/bin/sh\nwhile IFS= read -r line; do :; done\n".write(to: script, atomically: true, encoding: .utf8)
        try FileManager.default.setAttributes([.posixPermissions: 0o755], ofItemAtPath: script.path)
        guard case .opened(let id, _) = workspace.apply(action: .newTerminal).first else { throw CocoaError(.fileNoSuchFile) }
        let terminal = try Terminal.startForTab(
            tab: id, program: script.path, args: [], cwd: nil, cols: 80, rows: 24, cellWidth: 8, cellHeight: 16,
            listener: quiet)
        workspace.registerTerminal(id: id, terminal: terminal)
        return id
    }

    private func state(_ workspace: Workspace, _ id: Int64) -> RowState? {
        workspace.sidebar().sections.flatMap(\.rows).first { $0.id == id }?.state
    }

    func testACommandLineIsDoneBeforeALaunchAndALaunchIsLeftToTheApp() {
        XCTAssertNil(HookBridge.exitCodeIfCommandLine(["Wormhole"]))
        XCTAssertEqual(HookBridge.exitCodeIfCommandLine(["Wormhole", "no-such-command"]), 2)
    }

    func testTheShippedCommandLineTalksToTheSwiftApp() throws {
        let (workspace, handle) = try makeServer()
        defer { handle.stop() }
        _ = workspace.apply(action: .newTerminal)
        // `Wormhole status` as an agent's shell runs it, through the function the app's `main` calls.
        setenv("WORMHOLE_SOCKET", handle.socketPath, 1)
        defer { unsetenv("WORMHOLE_SOCKET") }
        XCTAssertEqual(HookBridge.exitCodeIfCommandLine(["Wormhole", "status"]), 0)
        // A terminal there is not is the app's refusal, as the exit code says.
        XCTAssertEqual(HookBridge.exitCodeIfCommandLine(["Wormhole", "status", "--terminal", "99"]), 1)
    }

    func testAnAgentsHookShowsInTheSidebar() async throws {
        let (workspace, handle) = try makeServer()
        defer { handle.stop() }
        let id = try agentTerminal(workspace)
        _ = workspace.apply(action: .newTerminal)  // another terminal is in front
        for _ in 0..<500 where workspace.sidebar().sections.flatMap(\.rows).first(where: { $0.id == id })?.agent == nil {
            try await Task.sleep(for: .milliseconds(20))
        }
        // The hook script's command line: `$WORMHOLE_BIN hook --agent claude <Event> <payload>`.
        setenv("WORMHOLE_SOCKET", handle.socketPath, 1)
        setenv("WORMHOLE_TAB_ID", String(id), 1)
        defer { unsetenv("WORMHOLE_SOCKET"); unsetenv("WORMHOLE_TAB_ID") }
        XCTAssertEqual(HookBridge.exitCodeIfCommandLine(["Wormhole", "hook", "--agent", "claude", "UserPromptSubmit", "{}"]), 0)
        for _ in 0..<500 where state(workspace, id) != .working { try await Task.sleep(for: .milliseconds(20)) }
        XCTAssertEqual(state(workspace, id), .working)
    }

    /// Codex's hooks (`json_hooks::CODEX`) run `hook --agent codex <Event>`: the prompt makes the terminal work, a question
    /// needs you, and the tool that runs once it is answered makes it work again.
    func testACodexHookFromTheShippedCommandLineMarksItsTerminalWorking() async throws {
        let (workspace, handle) = try makeServer()
        defer { handle.stop() }
        let id = try agentTerminal(workspace, named: "codex")
        _ = workspace.apply(action: .newTerminal)  // another terminal is in front
        setenv("WORMHOLE_SOCKET", handle.socketPath, 1)
        setenv("WORMHOLE_TAB_ID", String(id), 1)
        defer { unsetenv("WORMHOLE_SOCKET"); unsetenv("WORMHOLE_TAB_ID") }
        for (event, expected) in [("UserPromptSubmit", RowState.working), ("PermissionRequest", .needsYou), ("PostToolUse", .working)] {
            XCTAssertEqual(HookBridge.exitCodeIfCommandLine(["Wormhole", "hook", "--agent", "codex", event, "{}"]), 0)
            for _ in 0..<500 where state(workspace, id) != expected { try await Task.sleep(for: .milliseconds(20)) }
            XCTAssertEqual(state(workspace, id), expected, event)
        }
    }

    func testASecondLaunchAsksTheFirstToComeForwardAndIsRefused() async throws {
        let (_, handle) = try makeServer()
        defer { handle.stop() }
        let actions = handle.actions
        XCTAssertThrowsError(try makeServer()) { error in
            guard case HookError.AlreadyServed = error else { return XCTFail("\(error)") }
        }
        var heard: [HookAction] = []
        for await batch in actions {
            heard += batch
            if heard.contains(.activate) { break }
        }
        XCTAssertTrue(heard.contains(.activate))
        XCTAssertTrue(FileManager.default.fileExists(atPath: handle.socketPath))
    }
}
