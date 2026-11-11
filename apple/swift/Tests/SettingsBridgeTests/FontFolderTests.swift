import CoreText
import Foundation
@testable import SettingsBridge
import SettingsModel
import XCTest

/// The profile's fonts folder over the real core and CoreText: registered when the backend starts, followed while it runs.
@MainActor
final class FontFolderTests: BridgeTestCase {
    /// Whether CoreText gives `family` its own face (it hands back another family for a name it does not know).
    private func installed(_ family: String) -> Bool {
        CTFontCopyFamilyName(CTFontCreateWithName(family as CFString, 12, nil)) as String == family
    }

    private var font: URL {
        URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
            .deletingLastPathComponent().deletingLastPathComponent()
            .appendingPathComponent("crates/wormhole-core/assets/fonts/JetBrainsMono/JetBrainsMono-Italic.ttf")
    }

    func testAFontIsRegisteredAtStartAndFollowedAsTheFolderChanges() throws {
        try XCTSkipIf(installed("JetBrains Mono"), "the family is installed here, so registering it shows nothing")
        let fonts = folder.appendingPathComponent("fonts")
        try FileManager.default.createDirectory(at: fonts, withIntermediateDirectories: true)
        let file = fonts.appendingPathComponent("Mine.ttf")
        try FileManager.default.copyItem(at: font, to: file)

        let backend = try makeKit().backend
        XCTAssertEqual(backend.fontFolder.families, ["JetBrains Mono"], "read at start, before anything draws")
        XCTAssertTrue(installed("JetBrains Mono"), "and registered with CoreText for this process")
        let started = backend.fontFolder.revision
        var heard = 0
        let observation = backend.observe { heard += 1 }
        defer { _ = observation }

        try FileManager.default.removeItem(at: file)
        eventually("the removal to be followed") { backend.fontFolder.revision > started }
        XCTAssertTrue(backend.fontFolder.families.isEmpty)
        XCTAssertFalse(installed("JetBrains Mono"), "a font taken out of the folder is unregistered")
        XCTAssertGreaterThan(heard, 0, "the observers hear of fonts that moved though no value did")

        let removed = backend.fontFolder.revision
        try FileManager.default.copyItem(at: font, to: file)
        eventually("the new file to be followed") { backend.fontFolder.revision > removed }
        XCTAssertEqual(backend.fontFolder.families, ["JetBrains Mono"])
        XCTAssertTrue(installed("JetBrains Mono"))
        try FileManager.default.removeItem(at: file)
        eventually("the cleanup") { !self.installed("JetBrains Mono") }
    }

    func testAFolderWithNoFontsLeavesTheRevisionAtZero() throws {
        let backend = try makeKit().backend
        XCTAssertEqual(backend.fontFolder.revision, 0)
        XCTAssertTrue(backend.fontFolder.families.isEmpty)
    }
}
