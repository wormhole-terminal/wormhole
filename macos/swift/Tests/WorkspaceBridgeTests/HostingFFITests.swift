import Foundation
import WorkspaceModel
import WormholeBindings
import XCTest

@testable import WorkspaceBridge

/// Terminals in session hosts, over the real FFI and a real `wormhole-host` process (built into this tree's
/// `crates/wormhole-ffi/target` by `cargo test -p wormhole-ffi`, or `cargo build -p wormhole-host`): the program outlives the backend and the
/// next one attaches to it with its screen. Skipped without the host binary.
@MainActor
final class HostingFFITests: XCTestCase {
    private var data = ""
    private var host = ""

    override func setUpWithError() throws {
        host = URL(fileURLWithPath: #filePath).deletingLastPathComponent().appendingPathComponent("../../../../crates/wormhole-ffi/target/debug/wormhole-host")
            .standardizedFileURL.path
        try XCTSkipUnless(FileManager.default.isExecutableFile(atPath: host), "no host binary at \(host)")
        // /tmp: a socket's path must fit in 104 bytes.
        data = "/tmp/Wormhole-swift-hosting-\(UUID().uuidString.prefix(6))"
        try FileManager.default.createDirectory(atPath: data, withIntermediateDirectories: true)
        try Data(#"{"feature_flags": {"unlocked": true, "persistent_terminals": true}}"#.utf8)
            .write(to: URL(fileURLWithPath: data + "/config.json"))
        setenv("WORMHOLE_DATA_DIR", data, 1)
    }

    override func tearDown() async throws {
        guard !data.isEmpty else { return }
        // End All: whatever a failed test left running ends with the backend that attaches to it.
        let last = backend()
        last.restoreSaved()
        last.finishQuit(endHosts: true)
        try? FileManager.default.removeItem(atPath: data)
    }

    private func backend(hosting: Bool = true) -> RustWorkspaceBackend {
        // Each backend is a launch: the quit before it, in this same process, is over.
        HostConnect.launched()
        return RustWorkspaceBackend(
            setup: WorkspaceSetup(
                program: "/bin/cat", home: "/Users/ref", hosting: hosting ? HostingSetup(hostBinary: host) : nil))
    }

    private func screen(_ backend: RustWorkspaceBackend, _ id: TerminalID) -> String {
        backend.core.terminal(id: Int64(id))?.screenText() ?? ""
    }

    private func wait(_ what: String, _ done: () -> Bool) async throws {
        let deadline = Date().addingTimeInterval(15)
        while !done() {
            XCTAssertLessThan(Date(), deadline, "timed out waiting for \(what)")
            if Date() >= deadline { return }
            try await Task.sleep(for: .milliseconds(20))
        }
    }

    func testATerminalLeftRunningComesBackWithItsScreen() async throws {
        let first = backend()
        let store = WorkspaceStore(backend: first)
        XCTAssertEqual(first.restoreSaved(), 0, "nothing was saved yet")
        store.send(.newTerminal)
        // The host starts off the main actor, the pane blank meanwhile.
        XCTAssertEqual(store.hosting(of: 1), .connecting)
        try await wait("the host") { store.hosting(of: 1) != .connecting }
        guard case .persistent(let hostID)? = store.hosting(of: 1) else { return XCTFail("not in a host") }
        first.core.terminal(id: 1)?.write(bytes: Data("remember-me\n".utf8))
        try await wait("the echo") { screen(first, 1).contains("remember-me") }

        // Quit and keep: the hosts are let go of, the program runs on.
        store.finishQuit(endHosts: false)
        XCTAssertEqual(store.hosting(of: 1), .detached(hostID: hostID))

        let second = backend()
        let reopened = WorkspaceStore(backend: second)
        XCTAssertEqual(second.restoreSaved(), 1)
        XCTAssertEqual(reopened.state.live.count, 1)
        guard let id = reopened.state.live.first else { return }
        try await wait("the attach") { reopened.hosting(of: id) != .connecting }
        XCTAssertEqual(reopened.hosting(of: id), .persistent(hostID: hostID), "the same host")
        try await wait("the repaint") { screen(second, id).contains("remember-me") }

        // Closing a terminal on purpose ends its program (the window opens a fresh one in its place): the third
        // launch is offered the new host and not the old one.
        reopened.send(.close(id))
        try await wait("the fresh terminal's host") { reopened.state.live.allSatisfy { reopened.hosting(of: $0) != .connecting } }
        let third = backend()
        let entries = third.hosted?.launchEntries() ?? []
        XCTAssertEqual(entries.count, 1)
        XCTAssertFalse(entries.contains(.attach(hostID: hostID, folder: "/Users/ref")), "the closed terminal's host is gone")
    }

    func testWithoutPersistenceTerminalsStayInTheApp() {
        let plain = backend(hosting: false)
        let store = WorkspaceStore(backend: plain)
        store.send(.newTerminal)
        XCTAssertNil(store.hosting(of: 1), "persistence is off: the bar says nothing")
        XCTAssertEqual(plain.restoreSaved(), 0)
    }

    func testTheFlagOffKeepsTerminalsInTheApp() throws {
        try Data(#"{"feature_flags": {"unlocked": true, "persistent_terminals": false}}"#.utf8)
            .write(to: URL(fileURLWithPath: data + "/config.json"))
        let store = WorkspaceStore(backend: backend())
        store.send(.newTerminal)
        XCTAssertNil(store.hosting(of: 1))
    }

    func testTheCoresHostingMapsToTheWindowsStates() {
        XCTAssertEqual(HostingState(.local), .local)
        XCTAssertEqual(HostingState(.persistent(hostId: "a")), .persistent(hostID: "a"))
        XCTAssertEqual(HostingState(.detached(hostId: "a")), .detached(hostID: "a"))
        XCTAssertEqual(HostingState(.lost(hostId: "a")), .lost(hostID: "a"))
    }
}
