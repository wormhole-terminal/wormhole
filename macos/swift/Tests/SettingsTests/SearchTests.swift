import SettingsModel
import XCTest

@MainActor
final class SearchTests: XCTestCase {
    let search = SchemaSearch()

    func hits(_ query: String, values: [String: JSONValue] = SettingsSchema.bundled.defaults) -> [SearchHit] {
        search.search(query, values: values)
    }

    func testAnEmptyQueryFindsNothing() { XCTAssertTrue(hits("  ").isEmpty) }

    func testALabelThatStartsAWordComesFirstAndIsMarked() throws {
        let top = try XCTUnwrap(hits("line height").first)
        XCTAssertEqual(top.entry.id, "line-height")
        XCTAssertEqual(top.labelRange, 0..<4, "the first word found in the label")
    }

    func testEveryWordMustBeFound() {
        XCTAssertTrue(hits("scrollback zebra").isEmpty)
        XCTAssertFalse(hits("scrollback lines").isEmpty)
    }

    func testOutsideTheLabelAWordMustStartAWord() {
        XCTAssertFalse(hits("line").filter { $0.entry.id == "scrollback" }.isEmpty, "\"Lines kept\" starts with line")
        XCTAssertTrue(hits("eadline").isEmpty, "inside a word of the description is no match")
    }

    func testAConfigKeyFindsItsRow() {
        XCTAssertTrue(hits("scrollback_lines").contains { $0.entry.id == "scrollback" })
    }

    func testResultsComeInNavOrderWithEachSectionsBestFirst() {
        let found = hits("font")
        let order = SettingsSchema.bundled.sections.map(\.id)
        let indexes = found.map { order.firstIndex(of: $0.section) ?? 0 }
        XCTAssertEqual(indexes, indexes.sorted())
    }

    func testARowBehindAnOffSwitchIsNotFound() {
        var values = SettingsSchema.bundled.defaults
        values["sidebar_changes"] = .bool(false)
        XCTAssertTrue(hits("Files changed at once", values: values).isEmpty)
    }

    func testTheCountsPerSectionAddUp() {
        let backend = MockSettingsBackend()
        let model = SettingsModel(backend: backend, host: MockSettingsHost())
        model.query = "font"
        XCTAssertEqual(model.hitCounts.values.reduce(0, +), model.hits.count)
    }
}
