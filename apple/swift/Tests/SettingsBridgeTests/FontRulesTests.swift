import FontPickerModel
import Foundation
@testable import SettingsBridge
import SettingsModel
import XCTest

/// Core's font rules through the FFI, against the Swift mirror that a build without the core uses.
@MainActor
final class FontRulesTests: BridgeTestCase {
    private var catalog: FontCatalog {
        var catalog = FontCatalog()
        catalog.installed = ["Andale Mono", "Monaco", "PT Mono", "Menlo", "Mona Sans", ".SF NS", "JetBrains Mono", "Arial", "Zapfino"]
        catalog.bundled = ["JetBrains Mono"]
        catalog.custom = [CustomFamily(name: "Iosevka", monospace: true), CustomFamily(name: "Menlo", monospace: true)]
        catalog.monospace = ["Andale Mono", "Monaco", "PT Mono", "Menlo", "JetBrains Mono", "Iosevka"]
        return catalog
    }

    func testTheSwiftMirrorGivesWhatCoreGives() {
        let rust = RustFontRules(fontsDir: "/nonexistent/Wormhole-swift-fonts", bundledFiles: [])
        let local = LocalFontRules()
        var unclassified = catalog
        unclassified.monospace = nil
        for (catalog, query, mono) in [
            (catalog, "", true), (catalog, "", false), (catalog, "mon", false), (catalog, "  pt MONO ", true),
            (catalog, "o", true), (catalog, "nope", false), (unclassified, "", true), (catalog, "mona s", false),
        ] {
            XCTAssertEqual(
                local.choices(in: catalog, query: query, monospaceOnly: mono),
                rust.choices(in: catalog, query: query, monospaceOnly: mono), "query '\(query)' monospace \(mono)")
        }
    }

    func testTheBundledFamilyIsCores() {
        XCTAssertEqual(RustFontRules(fontsDir: "", bundledFiles: []).bundledFamily, LocalFontRules().bundledFamily)
    }

    func testCoreScansTheFontsFolder() throws {
        let folder = FileManager.default.temporaryDirectory.appendingPathComponent("Wormhole-swift-fonts-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: folder) }
        let font = URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent()
            .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
            .appendingPathComponent("crates/wormhole-core/assets/fonts/JetBrainsMono/JetBrainsMono-Italic.ttf")
        try FileManager.default.copyItem(at: font, to: folder.appendingPathComponent("Mine.ttf"))
        try Data("not a font".utf8).write(to: folder.appendingPathComponent("Broken.otf"))

        let scan = RustFontRules(fontsDir: folder.path, bundledFiles: []).scanCustomFonts()
        XCTAssertEqual(scan.families, [CustomFamily(name: "JetBrains Mono", monospace: true)])
        XCTAssertEqual(scan.files.map(\.lastPathComponent), ["Mine.ttf"])
        XCTAssertEqual(scan.skipped, 1, "a file that is not a font never hides the others")
    }

    func testTheHostHandsOutTheRustRules() throws {
        XCTAssertTrue(try makeKit().host.fontRules is RustFontRules)
    }
}
