import SettingsModel
import XCTest

@MainActor
final class ModelTests: XCTestCase {
    func make(_ overrides: [String: JSONValue] = [:]) -> (SettingsModel, MockSettingsBackend) {
        let backend = MockSettingsBackend(overrides: overrides)
        return (SettingsModel(backend: backend, host: MockSettingsHost()), backend)
    }

    func testAWriteReachesTheBackendAndComesBack() {
        let (model, backend) = make()
        model.set("blur", .bool(false))
        XCTAssertEqual(backend.writes, ["blur"])
        XCTAssertEqual(model.value("blur"), .bool(false), "read back through the backend's observation")
    }

    func testChangedIsAgainstTheDefault() throws {
        let (model, _) = make()
        let (_, _, blur) = try XCTUnwrap(model.schema.entry("blur"))
        XCTAssertFalse(model.isChanged(blur))
        XCTAssertEqual(model.changedCount, 0)
        model.set("blur", .bool(false))
        XCTAssertTrue(model.isChanged(blur))
        XCTAssertEqual(model.changedEntries.map(\.id), ["blur"])
        model.set("blur", .bool(true))
        XCTAssertEqual(model.changedCount, 0, "putting the value back is not a change")
    }

    func testResetPutsTheRowsKeysBack() throws {
        let (model, _) = make(["font_family": "Menlo", "font_size": 18])
        let (_, _, font) = try XCTUnwrap(model.schema.entry("font-terminal"))
        XCTAssertTrue(model.isChanged(font))
        model.reset(font)
        XCTAssertEqual(model.value("font_family"), "JetBrains Mono")
        XCTAssertEqual(model.value("font_size"), 14)
    }

    func testResetAllClearsTheChangedPage() {
        let (model, _) = make(["blur": false, "density": "dense", "opacity": 0.5])
        XCTAssertEqual(model.changedCount, 3)
        model.resetAll()
        XCTAssertEqual(model.changedCount, 0)
    }

    func testARowBehindAnOffSwitchIsNotOnTheChangedPage() {
        let (model, _) = make(["sidebar_changes": false, "sidebar_changes_remember": false])
        XCTAssertEqual(model.changedEntries.map(\.id), ["sidebar-changes"], "remember is hidden while the switch is off")
    }

    func testAnExternalWriteLandsLikeOurOwn() {
        let (model, backend) = make()
        backend.set("blur", to: .bool(false))
        XCTAssertEqual(model.value("blur"), .bool(false))
    }

    func testJumpingToAResultOpensItsPageAndMarksTheRow() throws {
        let (model, _) = make()
        model.query = "scrollback"
        let hit = try XCTUnwrap(model.hits.first)
        model.jump(to: hit)
        XCTAssertEqual(model.selection, "terminal")
        XCTAssertEqual(model.revealed, "scrollback")
        XCTAssertEqual(model.query, "")
    }

    func testTheNavListsEverySectionButProfileAndChanged() {
        let (model, _) = make()
        XCTAssertEqual(model.navSections.count, 16)
        XCTAssertFalse(model.navSections.contains { $0.id == "profile" })
    }
}
