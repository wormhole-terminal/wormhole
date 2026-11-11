import SettingsModel
import XCTest

final class SchemaTests: XCTestCase {
    let schema = SettingsSchema.bundled

    func testTheIndexHas18SectionsAndTheirRows() {
        XCTAssertEqual(schema.sections.count, 18)
        XCTAssertEqual(schema.sections.flatMap(\.entries).count, 124)
        XCTAssertEqual(schema.section("terminal")?.title, "Terminal")
        XCTAssertEqual(schema.section("sessions")?.title, "Terminals & sidebar")
    }

    func testEveryRowIdIsUnique() {
        let ids = schema.sections.flatMap(\.entries).map(\.id)
        XCTAssertEqual(ids.count, Set(ids).count)
    }

    func testEveryKeyARowWritesHasADefault() {
        for entry in schema.sections.flatMap(\.entries) {
            for key in entry.keys where !key.hasPrefix("motion.") {
                XCTAssertNotNil(schema.defaults[key], "\(entry.id): \(key)")
            }
        }
    }

    func testControlsDecodeWithTheirOptions() throws {
        let (_, _, density) = try XCTUnwrap(schema.entry("density"))
        guard case .segmented(let options, _, _) = density.control else { return XCTFail("\(density.control)") }
        XCTAssertEqual(options.map(\.label), ["Regular", "Compact", "Dense"])
        XCTAssertEqual(options.map(\.value), ["regular", "compact", "dense"])
        let (_, _, scrollback) = try XCTUnwrap(schema.entry("scrollback"))
        guard case .select(let lines, _) = scrollback.control else { return XCTFail("\(scrollback.control)") }
        XCTAssertEqual(lines.count, 6)
        XCTAssertEqual(lines[2].label, "10,000 lines")
        guard case .stepper(let stepper) = try XCTUnwrap(schema.entry("tab-width")).entry.control else { return XCTFail() }
        XCTAssertEqual(stepper.format(4), "4 spaces")
    }

    func testAToggleIsAnyOtherwiseUntouchedBoolean() throws {
        guard case .toggle = try XCTUnwrap(schema.entry("blur")).entry.control else { return XCTFail() }
        XCTAssertEqual(schema.defaults["blur"], .bool(true))
    }

    func testDefaultsKeepTheShortestFloat() {
        XCTAssertEqual(schema.defaults["opacity"], .number(0.8))
    }

    func testSessionsRowsThatNeedASwitchSayWhich() throws {
        let (_, _, scope) = try XCTUnwrap(schema.entry("sidebar-changes-scope"))
        XCTAssertFalse(scope.isShown(values: ["sidebar_changes": false]))
        XCTAssertTrue(scope.isShown(values: ["sidebar_changes": true]))
        let (_, _, plain) = try XCTUnwrap(schema.entry("sidebar-changes-plain"))
        XCTAssertFalse(plain.isShown(values: ["sidebar_changes": true, "sidebar_changes_scope": "repository"]))
        XCTAssertTrue(plain.isShown(values: ["sidebar_changes": true, "sidebar_changes_scope": "terminal"]))
    }
}
