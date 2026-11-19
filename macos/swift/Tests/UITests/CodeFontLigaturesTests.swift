import AppKit
import CoreText
@testable import DesignSystem
import XCTest

/// Settings → Fonts' Ligatures reaches the code text GPUI draws with `code_font_features` (`fonts.rs:219`): a project's
/// commands, the project commands window and a git problem's text, which the Swift app draws with `CodeFont.lineFont`.
final class CodeFontLigaturesTests: XCTestCase {
    private let family = "JetBrains Mono"
    /// Pairs the family joins (through `calt`).
    private let probe = "=> != <= ->"

    /// The repository's JetBrains Mono, registered for this process and taken back after the test.
    private func registerBundled() throws {
        let file = URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
            .deletingLastPathComponent().deletingLastPathComponent()
            .appendingPathComponent("crates/wormhole-core/assets/fonts/JetBrainsMono/JetBrainsMono-Regular.ttf")
        try XCTSkipUnless(FileManager.default.fileExists(atPath: file.path), "the bundled font is not where the repository keeps it")
        if CTFontManagerRegisterFontsForURL(file as CFURL, .process, nil) {
            addTeardownBlock { CTFontManagerUnregisterFontsForURL(file as CFURL, .process, nil) }
        }
    }

    private func glyphs(_ text: String, _ font: CTFont) -> [CGGlyph] {
        let line = CTLineCreateWithAttributedString(NSAttributedString(string: text, attributes: [kCTFontAttributeName as NSAttributedString.Key: font]))
        return (CTLineGetGlyphRuns(line) as? [CTRun] ?? []).flatMap { run in
            var glyphs = [CGGlyph](repeating: 0, count: CTRunGetGlyphCount(run))
            CTRunGetGlyphs(run, CFRange(location: 0, length: 0), &glyphs)
            return glyphs
        }
    }

    func testWithLigaturesOffALineOfCodeKeepsItsPairsApart() throws {
        try registerBundled()
        let joined = try XCTUnwrap(CTFont.face(of: family, size: 12), "the family is registered")
        let apart = CodeFont(family: family, size: 12, ligatures: false).lineFace(12)
        XCTAssertEqual(CTFontCopyFamilyName(apart) as String, family, "the same family, not a fallback")
        XCTAssertNotEqual(glyphs(probe, joined), glyphs(probe, apart), "the family joins the pairs, the line's face does not")
        // Apart is each character's own glyph, as the font maps it with no shaping.
        var characters = Array(probe.utf16)
        var plain = [CGGlyph](repeating: 0, count: characters.count)
        XCTAssertTrue(CTFontGetGlyphsForCharacters(joined, &characters, &plain, characters.count))
        XCTAssertEqual(glyphs(probe, apart), plain)
    }

    func testTheSystemsMonospacedFaceStandsInWithoutAFamily() {
        let face = CodeFont(family: nil, size: 12, ligatures: false).lineFace(12)
        XCTAssertTrue(CTFontGetSymbolicTraits(face).contains(.traitMonoSpace))
        XCTAssertEqual(CTFontGetSize(face), 12)
    }

    /// The window hands the font to its views by value: a change of the setting must be a change of the value.
    func testTheSettingIsPartOfTheFont() {
        XCTAssertNotEqual(CodeFont(ligatures: true), CodeFont(ligatures: false))
        XCTAssertTrue(CodeFont().ligatures, "on unless Settings say otherwise")
    }
}
