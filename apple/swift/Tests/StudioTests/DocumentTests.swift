@testable import DesignSystem
@testable import StudioModel
import XCTest

@MainActor
final class DocumentTests: XCTestCase {
    private var fixtures: StudioFixtures!

    override func setUpWithError() throws { fixtures = try StudioFixtures() }
    override func tearDown() { fixtures.remove() }

    func testOpeningABuiltInMakesACopyThatExtendsIt() throws {
        let doc = try fixtures.opening("Nord")
        XCTAssertEqual(doc.subject, .untitled(from: "Nord"))
        XCTAssertEqual(doc.name, "Nord Custom")
        XCTAssertEqual(doc.file.extends, "Nord")
        XCTAssertFalse(doc.isDirty)
        XCTAssertEqual(doc.tokens.terminal.background.hex, "#2e3440")
        XCTAssertEqual(fixtures.store.preview?.name, "Nord Custom", "every window previews the draft")
    }

    func testAnEditIsPreviewedAndUndone() throws {
        let doc = try fixtures.opening("Nord")
        XCTAssertTrue(doc.set("primary", to: "#ff6600"))
        XCTAssertEqual(fixtures.store.tokens.rgba(named: "primary")?.hex, "#ff6600")
        XCTAssertTrue(doc.isDirty)
        XCTAssertEqual(doc.undoLabel, "Change “primary”")
        doc.undo()
        XCTAssertNotEqual(fixtures.store.tokens.rgba(named: "primary")?.hex, "#ff6600")
        XCTAssertFalse(doc.isDirty)
        doc.redo()
        XCTAssertEqual(doc.tokens.rgba(named: "primary")?.hex, "#ff6600")
    }

    func testAValueRustRefusesStaysInTheFieldAndTheDraftKeepsItsLastGoodOne() throws {
        let doc = try fixtures.opening("Nord")
        doc.set("primary", to: "#ff6600")
        XCTAssertFalse(doc.set("primary", to: "primery"))
        XCTAssertEqual(doc.fieldError?.message, "Unknown token “primery” at 1:1.")
        XCTAssertEqual(doc.file.own("primary"), "#ff6600")
        XCTAssertTrue(doc.set("primary", to: "#00ff88"))
        XCTAssertNil(doc.fieldError)
    }

    func testAMetricMustBeANumberOfPixels() throws {
        let doc = try fixtures.opening("Nord")
        XCTAssertFalse(doc.set("metrics.radius", to: "round"))
        XCTAssertEqual(doc.fieldError?.message, "metrics.radius: “round” is not a number of pixels")
        XCTAssertTrue(doc.set("metrics.radius", to: "9px"))
        XCTAssertEqual(doc.row(for: "metrics.radius").text, "9px")
    }

    func testEmptyResetsToTheRule() throws {
        let doc = try fixtures.opening("Nord")
        doc.set("primary", to: "#ff6600")
        XCTAssertEqual(doc.origin(of: "primary"), .yours)
        XCTAssertTrue(doc.set("primary", to: "  "))
        XCTAssertEqual(doc.origin(of: "primary"), .workedOut)
        XCTAssertEqual(doc.undoLabel, "Reset “primary”")
    }

    func testRevertGoesBackToTheSavedFileAsOneUndoableStep() throws {
        let doc = try fixtures.opening("Nord")
        doc.set("primary", to: "#ff6600")
        doc.revertToSaved()
        XCTAssertFalse(doc.isDirty)
        doc.undo()
        XCTAssertTrue(doc.isDirty)
    }

    func testSavingWritesTheFileKeepsVersionsAndSetsNoticeThenUpdatesInPlace() throws {
        let doc = try fixtures.opening("Nord")
        doc.rename("Mine")
        doc.set("primary", to: "#ff6600")
        let url = try doc.save()
        XCTAssertEqual(url.lastPathComponent, "Mine.json")
        XCTAssertEqual(doc.subject, .own(url))
        XCTAssertFalse(doc.isDirty)
        XCTAssertTrue(try String(contentsOf: url, encoding: .utf8).contains("\"primary\": \"#ff6600\""))
        doc.set("primary", to: "#00ff88")
        try doc.save()
        let rows = doc.versionRows()
        XCTAssertEqual(rows.count, 2, "now, and the file the second save replaced")
        XCTAssertEqual(rows[0].summary, "primary")
        XCTAssertTrue(rows[1].pinned, "the first file kept is the original")
        doc.set("primary", to: "#112233")
        doc.restore(rows[1])
        XCTAssertEqual(doc.file.own("primary"), "#ff6600")
        XCTAssertEqual(doc.undoLabel, "Restore Version")
    }

    func testANameAnotherThemeHasIsNeverOverwritten() throws {
        try fixtures.folder.write("{ \"name\": \"Taken\" }", to: fixtures.folder.directory.appendingPathComponent("a.json"))
        let doc = try fixtures.opening("Nord")
        doc.rename("taken")
        XCTAssertThrowsError(try doc.save()) { XCTAssertEqual($0 as? SaveError, .nameTaken("taken")) }
    }

    func testAFileChangedByAnotherApplicationIsAskedAbout() throws {
        let doc = try fixtures.opening("Nord")
        doc.rename("Mine")
        let url = try doc.save()
        try "{ \"name\": \"Mine\", \"extends\": \"Dracula\" }".write(to: url, atomically: true, encoding: .utf8)
        doc.set("primary", to: "#ff6600")
        XCTAssertThrowsError(try doc.save()) { XCTAssertEqual($0 as? SaveError, .changedOnDisk) }
        XCTAssertNoThrow(try doc.save(overwriting: true))
    }

    func testACopyTakesAFreeName() throws {
        let doc = try fixtures.opening("Nord")
        doc.rename("Dusk")
        let first = try doc.saveCopy()
        let second = try doc.saveCopy()
        XCTAssertEqual(first.lastPathComponent, "Dusk copy.json")
        XCTAssertEqual(second.lastPathComponent, "Dusk copy 2.json")
        XCTAssertEqual(doc.subject, .untitled(from: "Nord"), "the draft itself stays as it was")
    }

    func testOwnFileOpensInPlaceAndTrashesAway() throws {
        let fixtures = try StudioFixtures(themes: ["Soltari.json": "// mine\n{ \"name\": \"Soltari\", \"extends\": \"Nord\", \"ui\": { \"primary\": \"#ff6600\" } }"])
        defer { fixtures.remove() }
        let doc = try fixtures.opening("Soltari")
        XCTAssertEqual(doc.subject, .own(fixtures.folder.directory.appendingPathComponent("Soltari.json")))
        XCTAssertEqual(doc.tokens.rgba(named: "primary")?.hex, "#ff6600")
        XCTAssertEqual(doc.origin(of: "primary"), .yours)
        XCTAssertEqual(doc.counts.yours, 1)
        doc.close()
        XCTAssertNil(fixtures.store.preview, "closing puts the person's pick back")
    }

    func testAPairOpensOnItsFirstVersionAndSwitches() throws {
        let pair = "{ \"name\": \"Pair\", \"extends\": \"Nord\", \"variants\": [{ \"name\": \"Dark\", \"appearance\": \"dark\" }, { \"name\": \"Light\", \"appearance\": \"light\", \"extends\": \"GitHub Light\" }] }"
        let fixtures = try StudioFixtures(themes: ["Pair.json": pair])
        defer { fixtures.remove() }
        let doc = try fixtures.opening("Pair Light")
        XCTAssertEqual(doc.shown, "Light")
        XCTAssertEqual(doc.appearance, .light)
        doc.switchVersion(to: "Dark")
        XCTAssertEqual(doc.appearance, .dark)
        XCTAssertEqual(fixtures.store.appearance, .dark)
    }
}
