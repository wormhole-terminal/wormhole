import Foundation
import SettingsBridge
import SettingsModel
import WormholeBindings
import XCTest

@MainActor
final class BackendTests: BridgeTestCase {
    func testAFreshProfileShowsTheSchemasDefaults() throws {
        let kit = try makeKit()
        for (key, value) in kit.schema.defaults { XCTAssertEqual(kit.backend.values[key], value, key) }
        XCTAssertNil(kit.backend.banner())
    }

    func testAWriteSticksReachesTheFileAndResets() throws {
        let kit = try makeKit()
        var changes = 0
        let observation = kit.backend.observe { changes += 1 }
        kit.backend.set("opacity", to: .number(0.5))
        XCTAssertEqual(kit.backend.values["opacity"], .number(0.5))
        XCTAssertEqual(changes, 1)
        let file = try String(contentsOfFile: kit.host.handle.paths().configFile, encoding: .utf8)
        XCTAssertTrue(file.contains("\"opacity\": 0.5"), file)
        kit.backend.set("opacity", to: .number(0.5))
        XCTAssertEqual(changes, 1, "a write that changes nothing is no news")

        kit.backend.reset(["opacity"])
        XCTAssertEqual(kit.backend.values["opacity"], kit.schema.defaults["opacity"])
        XCTAssertEqual(changes, 2)
        withExtendedLifetime(observation) {}
    }

    func testAValueCoreRefusesChangesNothingAndSaysWhy() throws {
        let kit = try makeKit()
        let before = kit.backend.values
        kit.backend.set("opacity", to: .string("dim"))
        XCTAssertEqual(kit.backend.values, before)
        XCTAssertTrue(kit.backend.lastRefusal?.hasPrefix("`opacity`: ") == true, kit.backend.lastRefusal ?? "nil")
        kit.backend.set("opacity", to: .number(0.6))
        XCTAssertNil(kit.backend.lastRefusal, "a write that sticks clears it")
    }

    func testAHandEditOfTheFileReachesTheModel() throws {
        let kit = try makeKit()
        var changes = 0
        let observation = kit.backend.observe { changes += 1 }
        let url = URL(fileURLWithPath: kit.host.handle.paths().configFile)
        try Data("{ \"opacity\": 0.4 }\n".utf8).write(to: url, options: .atomic)
        eventually("the file's wake") { kit.backend.values["opacity"] == .number(0.4) }
        XCTAssertGreaterThan(changes, 0)
        withExtendedLifetime(observation) {}
    }

    func testTheModelOverTheRealBackendReadsWhatStuck() throws {
        let kit = try makeKit()
        let model = SettingsModel(
            schema: kit.schema, backend: kit.backend, host: kit.host, searcher: kit.searcher, shortcuts: kit.shortcuts)
        model.set("diff_wrap", .bool(true))
        XCTAssertEqual(model.value("diff_wrap"), .bool(true))
        XCTAssertTrue(model.isChanged("diff_wrap"))
        XCTAssertEqual(model.changedCount, 1)
        model.resetAll()
        XCTAssertEqual(model.changedCount, 0)
    }
}
