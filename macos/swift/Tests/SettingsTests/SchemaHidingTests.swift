import SettingsModel
import XCTest

@MainActor
final class SchemaHidingTests: XCTestCase {
    /// What the beta hides: its one update channel, and the global command line tool.
    private let hidden: Set<String> = ["update-channel", "cli-tool"]

    func testHiddenRowsAreNotInThePagesNorFoundBySearchAndTheOthersStay() throws {
        let full = SettingsSchema.bundled
        let schema = full.hiding(hidden)
        XCTAssertNil(schema.entry("update-channel"))
        XCTAssertNil(schema.entry("cli-tool"))
        XCTAssertEqual(schema.sections.flatMap(\.entries).count, full.sections.flatMap(\.entries).count - 2)
        XCTAssertEqual(schema.sections.map(\.id), full.sections.map(\.id), "no page goes")
        XCTAssertNotNil(schema.entry("auto-update"), "the Updates page keeps its other rows")
        XCTAssertNotNil(schema.entry("control-write"), "and the Command line group its others")
        XCTAssertEqual(schema.defaults, full.defaults, "the setting itself stays, two-valued")
        // Search reads the same schema: the row is not offered as a result.
        let search = SchemaSearch(schema: schema)
        XCTAssertFalse(search.search("channel", values: schema.defaults).contains { $0.entry.id == "update-channel" })
        XCTAssertTrue(SchemaSearch(schema: full).search("channel", values: full.defaults).contains { $0.entry.id == "update-channel" })
    }

    func testHidingNothingIsTheSameSchema() {
        XCTAssertEqual(SettingsSchema.bundled.hiding([]).sections, SettingsSchema.bundled.sections)
    }
}
