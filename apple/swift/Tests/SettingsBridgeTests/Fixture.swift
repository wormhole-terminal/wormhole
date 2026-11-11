// A real core over throwaway folders: the data dir is set per test (the library refuses any folder not named
// `Wormhole-swift…`), agents' hooks go only under `WORMHOLE_AGENT_HOME`, and nothing reads the real home.

import Foundation
import SettingsBridge
import SettingsModel
import XCTest

@MainActor
class BridgeTestCase: XCTestCase {
    var folder: URL!

    override func setUp() async throws {
        folder = FileManager.default.temporaryDirectory.appendingPathComponent("Wormhole-swift-settings-\(UUID().uuidString)")
        let home = folder.appendingPathComponent("agent-home")
        try FileManager.default.createDirectory(at: home, withIntermediateDirectories: true)
        setenv("WORMHOLE_DATA_DIR", folder.path, 1)
        setenv("WORMHOLE_AGENT_HOME", home.path, 1)
    }

    override func tearDown() async throws {
        try? FileManager.default.removeItem(at: folder)
    }

    func makeKit(
        platform: RustSettingsHost.Platform = .init(), importHome: String? = nil
    ) throws -> SettingsBridge.Kit {
        try SettingsBridge.make(platform: platform, importHome: importHome ?? folder.appendingPathComponent("import-home").path)
    }

    /// Runs the main loop until `condition` holds: the wake and the host's reads arrive on it from other threads.
    func eventually(_ what: String, timeout: TimeInterval = 8, _ condition: () -> Bool) {
        let deadline = Date().addingTimeInterval(timeout)
        while !condition() {
            if Date() > deadline { return XCTFail("timed out waiting for \(what)") }
            RunLoop.main.run(until: Date().addingTimeInterval(0.05))
        }
    }
}
