// What a family's faces can do, for the preview's chips: its styles, whether it has the glyphs a terminal draws
// with and whether it joins `=>` and the like. Each asks CoreText of the family alone, never of its fallbacks.

import CoreText
import FontPickerModel
import Foundation

extension CoreTextFonts {
    public func styles(of family: String) -> [String] {
        let descriptor = CTFontDescriptorCreateWithAttributes([kCTFontFamilyNameAttribute: family] as CFDictionary)
        let faces = (CTFontDescriptorCreateMatchingFontDescriptors(descriptor, nil) as? [CTFontDescriptor]) ?? []
        return faces.compactMap { CTFontDescriptorCopyAttribute($0, kCTFontStyleNameAttribute) as? String }
    }

    public func covers(_ family: String, _ scalars: [Unicode.Scalar]) -> Bool {
        guard let font = Self.font(named: family, size: 12) else { return false }
        return scalars.allSatisfy { scalar in
            var characters = Array(String(scalar).utf16)
            var glyphs = [CGGlyph](repeating: 0, count: characters.count)
            return CTFontGetGlyphsForCharacters(font, &characters, &glyphs, characters.count)
        }
    }

    /// Ligatures are a font's own: the same text shaped with `calt` and `liga` on, then off, gives other glyphs.
    /// (Programming fonts join through contextual alternates, others through standard ligatures.)
    public func hasLigatures(_ family: String) -> Bool {
        guard let font = Self.font(named: family, size: 12) else { return false }
        let off = CTFontDescriptorCreateWithAttributes(
            [
                kCTFontFeatureSettingsAttribute: ["calt", "liga"].map {
                    [kCTFontOpenTypeFeatureTag: $0, kCTFontOpenTypeFeatureValue: 0] as [CFString: Any]
                }
            ] as CFDictionary)
        let plain = CTFontCreateCopyWithAttributes(font, 12, nil, off)
        let text = "=> -> != == <= >= |> www"
        return Self.glyphs(text, font) != Self.glyphs(text, plain)
    }

    private static func glyphs(_ text: String, _ font: CTFont) -> [CGGlyph] {
        let line = CTLineCreateWithAttributedString(NSAttributedString(string: text, attributes: [kCTFontAttributeName as NSAttributedString.Key: font]))
        return (CTLineGetGlyphRuns(line) as? [CTRun] ?? []).flatMap { run in
            var glyphs = [CGGlyph](repeating: 0, count: CTRunGetGlyphCount(run))
            CTRunGetGlyphs(run, CFRange(location: 0, length: 0), &glyphs)
            return glyphs
        }
    }
}
