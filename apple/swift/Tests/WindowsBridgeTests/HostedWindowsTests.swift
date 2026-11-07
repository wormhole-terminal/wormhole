import Foundation
import WindowsModel
import WorkspaceModel
import XCTest

@testable import WindowsBridge
@testable import WorkspaceBridge

/// Terminals in session hosts across two windows, over the real FFI and a real `wormhole-host` (as `HostingFFITests`):
/// each window gets back its own terminals under their own ids, attached to their own hosts, and no window takes
/// another's host for an orphan. Skipped without the host binary.
@MainActor
final class HostedWindowsTests: XCTestCase {
    private var data = ""
    private var host = ""

    override func setUpWithError() throws {
        host = URL(fileURLWithPath: #filePath).deletingLastPathComponent().appendingPathComponent("../../../target/debug/wormhole-host")
            .standardizedFileURL.path
        try XCTSkipUnless(FileManager.default.isExecutableFile(atPath: host), "no host binary at \(host)")
        data = "/tmp/Wormhole-swift-hostedwin-\(UUID().uuidString.prefix(6))"
        try FileManager.default.createDirectory(atPath: data, withIntermediateDirectories: true)
        try Data(#"{"feature_flags": {"unlocked": true, "persistent_terminals": true}}"#.utf8)
            .write(to: URL(fileURLWithPath: data + "/config.json"))
        setenv("WORMHOLE_DATA_DIR", data, 1)
    }

    override func tearDown() async throws {
        guard !data.isEmpty else { return }
        // End All: whatever a failed test left running ends with the window that attaches to it.
        if let windows = try? RustWindowsBackend() {
            for saved in windows.saved() {
                let (store, _) = window(windows, saved.id, first: false)
                store.restore()
                store.finishQuit(endHosts: true)
            }
        }
        try? FileManager.default.removeItem(atPath: data)
    }

    private func window(_ windows: RustWindowsBackend, _ id: Int, first: Bool) -> (WorkspaceStore, RustWorkspaceBackend) {
        let hosting = HostingSetup(hostBinary: host, window: Int64(id), adoptsOrphans: first)
        let backend = windows.workspace(window: id, setup: WorkspaceSetup(program: "/bin/cat", home: "/Users/ref", hosting: hosting))
        return (WorkspaceStore(backend: backend), backend)
    }

    func testEachWindowGetsItsOwnHostedTerminalsBackUnderTheirIds() throws {
        var hostOf: [Int: String] = [:]
        do {
            let windows = try RustWindowsBackend()
            let (a, aBackend) = window(windows, windows.allocWindowId(), first: true)
            let (b, bBackend) = window(windows, windows.allocWindowId(), first: false)
            a.send(.newTerminal)
            b.send(.newTerminal)
            b.send(.newTerminal)
            XCTAssertEqual(a.state.live, [1])
            XCTAssertEqual(b.state.live, [2, 3])
            for (store, id) in [(a, 1), (b, 2), (b, 3)] {
                guard case .persistent(let host)? = store.hosting(of: id) else { return XCTFail("terminal \(id) is not in a host") }
                hostOf[id] = host
            }
            XCTAssertTrue(windows.save(window: 1, workspace: aBackend, frame: nil))
            XCTAssertTrue(windows.save(window: 2, workspace: bBackend, frame: nil))
            // Quit and keep: the hosts are let go of, the programs run on.
            a.finishQuit(endHosts: false)
            b.finishQuit(endHosts: false)
        }
        let windows = try RustWindowsBackend()
        XCTAssertEqual(windows.saved().map(\.id).sorted(), [1, 2])
        let (a, _) = window(windows, 1, first: true)
        let (b, _) = window(windows, 2, first: false)
        a.restore()
        b.restore()
        XCTAssertEqual(a.state.live, [1])
        XCTAssertEqual(b.state.live, [2, 3])
        for (store, id) in [(a, 1), (b, 2), (b, 3)] {
            XCTAssertEqual(store.hosting(of: id), .persistent(hostID: hostOf[id] ?? ""), "terminal \(id) is in its own host again")
        }
        XCTAssertTrue(a.state.groups.isEmpty, "window 1 did not take window 2's hosts for orphans")
    }
}
