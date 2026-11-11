import AuxModel
import XCTest

@testable import AuxBridge

@MainActor
final class LicensesAndImportTests: XCTestCase {
    func testTheBundledListLoadsThroughTheListsOwnReader() throws {
        let list = try AuxBridge.makeLicenseSource().load()
        XCTAssertGreaterThan(list.packages.count, 100)
        let serde = try XCTUnwrap(list.packages.first { $0.name == "serde" })
        XCTAssertFalse(list.texts(of: serde).isEmpty)
        XCTAssertTrue(list.search("serde").contains { list.packages[$0].name == "serde" })
        XCTAssertTrue(list.packages.contains { $0.kind == .font })
    }

    func testAnotherTerminalsSettingsAreFoundAndApplied() async throws {
        let scratch = try Scratch()
        defer { scratch.remove() }
        try scratch.write("font-size = 15\nfont-family = Menlo\n", to: "home/.config/ghostty/config")
        let home = scratch.root.appendingPathComponent("home")
        let offers = try await ImportBridge.scan(home: home)
        let offer = try XCTUnwrap(offers.first)
        XCTAssertEqual(offer.source, "ghostty")
        XCTAssertEqual(offer.name, "Ghostty")
        XCTAssertTrue(offer.rows.contains { $0.setting == "font_size" && $0.tickedByDefault })
        let taken = try await ImportBridge.apply(source: "ghostty", home: home, ticked: ["font_size"], dataDir: scratch.data)
        XCTAssertEqual(taken, 1)
        let text = try String(contentsOf: scratch.data.appendingPathComponent("config.json"), encoding: .utf8)
        XCTAssertTrue(text.contains("15"), text)
    }
}
