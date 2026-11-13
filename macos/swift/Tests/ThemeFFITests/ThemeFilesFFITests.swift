@testable import DesignSystem
import Foundation
import SettingsModel
@testable import SettingsUI
import StudioBridge
@testable import StudioModel
import StudioUI
import UniFFISource
import WormholeBindings
import XCTest

/// The theme files, picks and Studio over `wormhole-core`'s real rules, through the FFI.
@MainActor
final class ThemeFilesFFITests: XCTestCase {
    private var folder: URL!

    override func setUpWithError() throws {
        folder = FileManager.default.temporaryDirectory.appendingPathComponent("wh-theme-files-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true)
        let mine = "{ \"name\": \"Mine\", \"extends\": \"Nord\", \"ui\": { \"primary\": \"#ff6600\" } }\n"
        try mine.write(to: folder.appendingPathComponent("Mine.json"), atomically: true, encoding: .utf8)
    }

    override func tearDownWithError() throws { try? FileManager.default.removeItem(at: folder) }

    private func store() -> ThemeStore {
        let store = ThemeStore(follows: false)
        store.systemAppearance = .dark
        store.resolver = FfiThemeResolver()
        store.themesDirectory = folder
        return store
    }

    private func environment() -> StudioEnvironment {
        StudioEnvironment(
            resolver: FfiThemeResolver(), folder: FfiThemeFolder(directory: folder), knowledge: GalleryKnowledge(),
            files: FfiStudioFiles(directory: folder))
    }

    // MARK: The folder and the editor's help, through core

    func testASaveKeepsWhatItReplacedAsAVersionAndCoreReadsItBack() throws {
        let themes = FfiThemeFolder(directory: folder)
        let file = folder.appendingPathComponent("Mine.json")
        let before = try themes.read(file)
        try themes.write("{ \"name\": \"Mine\" }\n", to: file)
        let kept = try XCTUnwrap(themes.versions(of: file).first)
        XCTAssertTrue(kept.pinned, "a file written by hand is kept as the pinned original")
        XCTAssertEqual(try themes.readVersion(kept), before)
        XCTAssertEqual(themes.files().map(\.lastPathComponent), ["Mine.json"])
    }

    func testCoreOffersFunctionsAndNamesAtTheCaretAndFixesAPairThatFallsShort() throws {
        let files = FfiStudioFiles(directory: folder)
        let found = try XCTUnwrap(files.completions("mix(pri", caret: 7, target: "primary.hover"))
        XCTAssertEqual(found.range, 4..<7)
        XCTAssertTrue(found.items.contains { $0.label == "primary" && !$0.function })
        XCTAssertNil(files.completions("primary", caret: 7, target: "primary.hover"), "a finished name offers nothing")
        let weak = "{ \"name\": \"Weak\", \"extends\": \"Nord\", \"ui\": { \"text.muted\": \"#333847\" } }"
        let issue = try XCTUnwrap(files.contrastIssues(weak, shown: nil).first { $0.check.fg == "text.muted" })
        XCTAssertTrue(try XCTUnwrap(issue.fix).expression.hasPrefix("readable(#333847"))
    }

    // MARK: Picks (audit gap 7)

    func testAUserThemePickedInTheConfigIsShownWhateverNamesIt() throws {
        let store = store()
        let id = try XCTUnwrap(store.userThemes.first { $0.name == "Mine" }).id
        for pick in ["Mine", id, folder.appendingPathComponent("Mine.json").path] {
            ThemeSync.apply(["theme": .string("Wormhole Dark")], to: store)
            ThemeSync.apply(["theme": .string(pick)], to: store)
            XCTAssertEqual(store.theme.name, "Mine", "a pick by \(pick)")
            XCTAssertEqual(store.tokens.rgba(named: "primary")?.hex, "#ff6600")
            XCTAssertNil(store.problem)
        }
        ThemeSync.apply(["theme": .string("No such theme")], to: store)
        XCTAssertNotNil(store.problem, "a pick that does not resolve is reported")
        XCTAssertEqual(store.theme.id, "wormhole.dark", "and the default shows until it is fixed")
    }

    func testEditingTheFileInUseRecoloursTheAppWhenTheFollowerAsks() throws {
        let store = store()
        ThemeSync.apply(["theme": .string("Mine")], to: store)
        XCTAssertEqual(store.tokens.rgba(named: "primary")?.hex, "#ff6600")
        store.refreshIfThemeFilesChanged()  // nothing changed: nothing is re-read
        let edited = "{ \"name\": \"Mine\", \"extends\": \"Nord\", \"ui\": { \"primary\": \"#00aa44\" } }\n"
        try edited.write(to: folder.appendingPathComponent("Mine.json"), atomically: true, encoding: .utf8)
        store.refreshIfThemeFilesChanged()
        XCTAssertEqual(store.tokens.rgba(named: "primary")?.hex, "#00aa44")
    }

    func testAProjectThemeShowsInPlaceOfThePickAndKeepsTheChoices() {
        let store = store()
        store.projectTheme = "{ \"name\": \"App\", \"extends\": \"Nord\", \"ui\": { \"primary\": \"#112233\" } }"
        XCTAssertEqual(store.theme.name, "App")
        XCTAssertEqual(store.tokens.rgba(named: "primary")?.hex, "#112233")
        store.settings.primary = "#ff00ff"
        XCTAssertEqual(store.tokens.rgba(named: "primary")?.hex, "#ff00ff", "the person's colour goes over the project's theme")
        store.projectTheme = nil
        XCTAssertNotEqual(store.theme.name, "App")
        store.projectTheme = "{ not a theme"
        XCTAssertNotEqual(store.theme.name, "App", "a project theme that does not resolve is not shown")
    }

    func testTheDraftsKeyTableIsCores() {
        let core = studioKeyTable().map { "\($0.block).\($0.old).\($0.new)" }.sorted()
        XCTAssertEqual(StudioKeys.table.map { "\($0.block).\($0.old).\($0.new)" }.sorted(), core)
    }

    // MARK: The Studio on version 1 files (gaps 12, 21)

    func testACopyOfABuiltInCarriesItsValuesInABaseAndGoesBackToThemOnReset() throws {
        let doc = try StudioDocument.open(theme: "Nord", environment: environment(), store: store())
        XCTAssertTrue(doc.file.isV1)
        XCTAssertEqual(doc.name, "Untitled (from Nord)")
        XCTAssertFalse(doc.isDirty)
        XCTAssertEqual(doc.origin(of: "terminal.background"), .base)
        XCTAssertEqual(doc.update.from, "Nord")
        XCTAssertTrue(doc.set("primary", to: "#ff6600"))
        XCTAssertEqual(doc.origin(of: "primary"), .yours)
        XCTAssertEqual(doc.file.own("primary"), "#ff6600")
        XCTAssertTrue(doc.set("terminal.cursor_text", to: "#010101"), "a key version 1 spells differently")
        XCTAssertTrue(doc.file.root["terminal"]?.object?["cursor.text"] != nil)
        doc.reset("primary")
        XCTAssertNotEqual(doc.origin(of: "primary"), .yours)
    }

    func testSavingANewCopyWritesVersionOneWithAnIdAndSetsThePickOnce() throws {
        let store = store()
        let picks = RecordingPicks()
        store.picks = picks
        let doc = try StudioDocument.open(theme: "Nord", environment: environment(), store: store)
        doc.rename("Dusk")
        doc.set("primary", to: "#ff6600")
        let url = try doc.save()
        let text = try String(contentsOf: url, encoding: .utf8)
        XCTAssertTrue(text.contains("\"version\": 1") && text.contains("\"id\": \"dusk\"") && text.contains("\"base\""), text)
        XCTAssertEqual(picks.saved.map(\.value), [url.path], "the theme is a pick of its appearance by its file")
        XCTAssertEqual(picks.saved.first?.dark, true)
        XCTAssertEqual(doc.notice?.text.contains("set as your theme"), false, "the fake did not change a pick")
        // Opened again, it is edited in place, whatever format wrote it.
        let again = try StudioDocument.open(theme: "Dusk", environment: environment(), store: store)
        XCTAssertEqual(again.subject, .own(url))
        XCTAssertEqual(again.origin(of: "primary"), .yours)
        XCTAssertEqual(again.origin(of: "terminal.background"), .base)
    }

    func testAnOlderFileIsEditedInPlaceAndConvertedByItsFirstSave() throws {
        let store = store()
        let doc = try StudioDocument.open(theme: "Mine", environment: environment(), store: store)
        XCTAssertFalse(doc.file.isV1)
        doc.set("primary", to: "#00aa44")
        let url = try doc.save()
        let text = try String(contentsOf: url, encoding: .utf8)
        XCTAssertTrue(text.contains("\"version\": 1") && text.contains("#00aa44"), text)
        XCTAssertTrue(doc.notice?.text.contains("older releases") == true, "said once, on the save that does it")
        XCTAssertEqual(doc.versionRows().count, 2, "the file it replaced is kept")
    }

    func testASingleThemeBecomesAPairAndExportsOneAppearance() throws {
        let doc = try StudioDocument.open(theme: "Wormhole Dark", environment: environment(), store: store())
        XCTAssertEqual(doc.missingAppearance, .light)
        doc.addAppearance(.light)
        XCTAssertEqual(doc.file.versions, ["light", "dark"])
        XCTAssertEqual(doc.shown, "light")
        XCTAssertNil(doc.missingAppearance)
        let exported = doc.exportJSON
        XCTAssertFalse(exported.contains("\"version\""), "Export is the format every release reads")
        XCTAssertEqual(doc.undoLabel, "Add Appearance")
    }

    func testAnUpdateFromBringsABaseUpToDateAsOneStep() throws {
        let doc = try StudioDocument.open(theme: "Nord", environment: environment(), store: store())
        XCTAssertEqual(doc.update.state, .current)
        var stale = doc.file
        stale.root["base"] = .object(
            (stale.root["base"]?.object ?? [:]).merging(["ui": .object(["primary": .string("#010203")])]) { _, new in new })
        try doc.take("stale") { $0 = stale }
        let rows = try XCTUnwrap(doc.updateRows())
        XCTAssertTrue(rows.changes.contains { $0.key == "ui.primary" && $0.old == "#010203" })
        doc.applyUpdate()
        XCTAssertEqual(doc.update.state, .current)
        XCTAssertEqual(doc.undoLabel, "Update from Base")
    }

    func testANewerFileIsNeverWrittenBack() throws {
        // A whole version 1 file from the core (a version 1 theme carries every terminal colour), as a later release would
        // have written it: a higher version and a block this one does not know.
        let copy = try XCTUnwrap(FfiStudioFiles(directory: folder).copyTheme(from: "Nord", name: "Future"))
        var object = try XCTUnwrap(JSONSerialization.jsonObject(with: Data(copy.utf8)) as? [String: Any])
        object["version"] = 9
        object["surprise"] = ["x": 1]
        let newer = try XCTUnwrap(String(data: JSONSerialization.data(withJSONObject: object, options: [.prettyPrinted, .sortedKeys]), encoding: .utf8))
        try newer.write(to: folder.appendingPathComponent("Future.json"), atomically: true, encoding: .utf8)
        let store = store()
        store.reloadThemes()
        let doc = try StudioDocument.open(theme: "Future", environment: environment(), store: store)
        XCTAssertTrue(doc.isNewer)
        doc.set("primary", to: "#ff6600")
        XCTAssertThrowsError(try doc.save()) { XCTAssertEqual($0 as? SaveError, .newer) }
        XCTAssertEqual(try String(contentsOf: folder.appendingPathComponent("Future.json"), encoding: .utf8), newer)
    }
}

@MainActor
private final class RecordingPicks: ThemePicking {
    var saved: [(value: String, dark: Bool)] = []
    func pick(_ value: String, dark: Bool) {}
    func pickAfterSave(_ value: String, dark: Bool, path: String) -> Bool {
        saved.append((value, dark))
        return false
    }
    func renamePicks(_ renames: [(old: String, new: String)]) {}
    func followSystem(light: String, dark: String) {}
    func selects(_ setting: String, path: String) -> Bool { false }
}
