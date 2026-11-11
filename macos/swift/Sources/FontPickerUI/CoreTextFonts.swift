// CoreText's side of the picker (`platform/macos/fonts.rs`): the installed families and a family's traits,
// classified by the same rules as the GPUI app's, and font files made available to this process. Everything
// here is thread-safe, which is what lets the library ask off the main actor.

import CoreText
import FontPickerModel
import Foundation

public struct CoreTextFonts: SystemFonts {
    public init() {}

    public func families() -> [String] {
        ((CTFontManagerCopyAvailableFontFamilyNames() as? [String]) ?? []).sorted()
    }

    public func register(_ files: [URL]) {
        guard !files.isEmpty else { return }
        // An already registered file is not an error worth reporting: loading one twice changes nothing.
        CTFontManagerRegisterFontURLs(files as CFArray, .process, true, nil)
    }

    public func traits(of families: [String]) -> [String: FontTraits] {
        Dictionary(uniqueKeysWithValues: families.compactMap { name in traits(of: name).map { (name, $0) } })
    }

    /// A family by its default face. The fixed-pitch flag alone misses some coding fonts, so equal advances for
    /// `iMW0` count as monospace too; a face that lacks those letters is a symbol font. CoreText hands back a
    /// default font for a family it does not know, which is not an answer.
    func traits(of family: String) -> FontTraits? {
        guard let font = Self.font(named: family, size: 12) else { return nil }
        let traits = CTFontGetSymbolicTraits(font)
        let stylistic = CTFontStylisticClass(rawValue: traits.rawValue & CTFontSymbolicTraits.classMaskTrait.rawValue)
        let ornamental = stylistic == .symbolicClass || stylistic == .ornamentalsClass
        var characters: [UniChar] = Array("iMW0".utf16)
        var glyphs = [CGGlyph](repeating: 0, count: 4)
        let mapped = CTFontGetGlyphsForCharacters(font, &characters, &glyphs, 4)
        var advances = [CGSize](repeating: .zero, count: 4)
        if mapped { CTFontGetAdvancesForGlyphs(font, .horizontal, glyphs, &advances, 4) }
        let equal = mapped && advances[0].width > 0 && advances.allSatisfy { abs($0.width - advances[0].width) < 0.01 }
        return FontTraits(monospace: traits.contains(.traitMonoSpace) || equal, symbolic: ornamental || !mapped)
    }

    /// The font of a family, or nil when CoreText substituted another family for it.
    static func font(named family: String, size: CGFloat) -> CTFont? {
        let descriptor = CTFontDescriptorCreateWithAttributes([kCTFontFamilyNameAttribute: family] as CFDictionary)
        let font = CTFontCreateWithFontDescriptor(descriptor, size, nil)
        return CTFontCopyFamilyName(font) as String == family ? font : nil
    }
}
