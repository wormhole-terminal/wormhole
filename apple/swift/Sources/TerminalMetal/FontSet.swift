// The fonts a terminal draws with and the cell they imply. Everything here is in device pixels: the
// fonts are created at `points * scale`, so a glyph rasterises at the size it is drawn and the cell is a
// whole number of pixels (a fractional cell would put every glyph at a different subpixel offset, which is
// what the atlas avoids by not keeping subpixel variants).

import CoreGraphics
import CoreText
import Foundation

/// The four faces a cell can ask for: SGR 1 and SGR 3 pick one.
enum FontStyle: Int, CaseIterable {
    case regular = 0
    case bold
    case italic
    case boldItalic

    init(bold: Bool, italic: Bool) {
        self = bold ? (italic ? .boldItalic : .bold) : (italic ? .italic : .regular)
    }
}

struct CellMetrics: Equatable {
    var width: Int
    var height: Int
    /// From the top of the cell to the baseline.
    var baseline: Int
    /// The top of an underline, from the top of the cell; and its thickness.
    var underlineTop: Int
    var lineThickness: Int
    /// The top of a strikethrough, from the top of the cell.
    var strikeTop: Int
}

final class FontSet {
    let family: String
    let pointSize: CGFloat
    let scale: CGFloat
    let metrics: CellMetrics

    private var interned: [CTFont] = []
    private var ids: [String: UInt16] = [:]

    init(family: String, pointSize: CGFloat, scale: CGFloat) {
        self.family = family
        self.pointSize = pointSize
        self.scale = scale
        let size = pointSize * scale
        let regular = CTFontCreateWithName(family as CFString, size, nil)
        func variant(_ traits: CTFontSymbolicTraits) -> CTFont {
            CTFontCreateCopyWithSymbolicTraits(regular, size, nil, traits, traits) ?? regular
        }
        // `FontStyle`'s order: regular, bold, italic, bold italic. They are interned first, so a face's id is
        // its style's raw value.
        for font in [regular, variant(.traitBold), variant(.traitItalic), variant(.traitBold.union(.traitItalic))] {
            interned.append(font)
        }
        for (index, font) in interned.enumerated() { ids[Self.key(font)] = UInt16(index) }

        var glyph = CGGlyph(0)
        var m: UniChar = 0x4D
        CTFontGetGlyphsForCharacters(regular, &m, &glyph, 1)
        var advance = CGSize.zero
        CTFontGetAdvancesForGlyphs(regular, .default, &glyph, &advance, 1)
        let ascent = CTFontGetAscent(regular), descent = CTFontGetDescent(regular), leading = CTFontGetLeading(regular)
        let height = Int((ascent + descent + leading).rounded(.up))
        let baseline = Int(ascent.rounded(.up) + (leading / 2).rounded(.down))
        let thickness = max(1, Int(CTFontGetUnderlineThickness(regular).rounded()))
        let underlineBelow = max(1, Int(abs(CTFontGetUnderlinePosition(regular)).rounded()))
        let xHeight = CTFontGetXHeight(regular)
        metrics = CellMetrics(
            width: max(1, Int(advance.width.rounded())), height: max(1, height), baseline: baseline,
            underlineTop: min(height - thickness, baseline + underlineBelow - thickness / 2), lineThickness: thickness,
            strikeTop: baseline - Int((xHeight / 2).rounded()) - thickness / 2)
    }

    func font(for style: FontStyle) -> CTFont { interned[style.rawValue] }
    func font(id: UInt16) -> CTFont { interned[Int(id)] }

    /// A small id for a font CoreText picked for a run (a fallback for CJK, emoji, symbols), for the atlas key.
    func id(of font: CTFont) -> UInt16 {
        let key = Self.key(font)
        if let id = ids[key] { return id }
        let id = UInt16(interned.count)
        interned.append(font)
        ids[key] = id
        return id
    }

    private static func key(_ font: CTFont) -> String {
        "\(CTFontCopyPostScriptName(font))@\(CTFontGetSize(font))"
    }

    /// Whether the font draws colour bitmaps or layers (Apple Color Emoji).
    static func hasColorGlyphs(_ font: CTFont) -> Bool {
        CTFontGetSymbolicTraits(font).contains(.traitColorGlyphs)
    }
}
