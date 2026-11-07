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

/// Where the four faces come from.
public enum FontSpec: Sendable, Hashable {
    /// An installed family; bold and italic are its symbolic-trait variants.
    case system(family: String)
    /// Four font files, registered for this process so CoreText also finds them for fallback.
    case files(regular: URL, bold: URL, italic: URL, boldItalic: URL)

    /// The bundled JetBrains Mono, from the folder that holds its four TTFs.
    public static func jetBrainsMono(directory: URL) -> FontSpec {
        .files(
            regular: directory.appendingPathComponent("JetBrainsMono-Regular.ttf"),
            bold: directory.appendingPathComponent("JetBrainsMono-Bold.ttf"),
            italic: directory.appendingPathComponent("JetBrainsMono-Italic.ttf"),
            boldItalic: directory.appendingPathComponent("JetBrainsMono-BoldItalic.ttf"))
    }
}

final class FontSet {
    let pointSize: CGFloat
    let scale: CGFloat
    let metrics: CellMetrics

    private var interned: [CTFont] = []
    private var ids: [String: UInt16] = [:]

    /// `settings.lineHeight` is a multiple of the font size (1.3 in the GPUI app); without it the font's own line.
    init(settings: TerminalFontSettings, scale: CGFloat) {
        pointSize = CGFloat(settings.pointSize)
        self.scale = scale
        let size = pointSize * scale
        let lineHeight = settings.lineHeight.map { CGFloat($0) }
        let plain: [CTFont]
        switch settings.font {
        case .system(let family):
            let regular = CTFontCreateWithName(family as CFString, size, nil)
            func variant(_ traits: CTFontSymbolicTraits) -> CTFont {
                CTFontCreateCopyWithSymbolicTraits(regular, size, nil, traits, traits) ?? regular
            }
            plain = [regular, variant(.traitBold), variant(.traitItalic), variant(.traitBold.union(.traitItalic))]
        case .files(let regular, let bold, let italic, let boldItalic):
            plain = [regular, bold, italic, boldItalic].map { Self.load($0, size: size) }
        }
        let faces = plain.map { Self.configured($0, settings: settings, size: size) }
        let regular = faces[0]
        // `FontStyle`'s order: regular, bold, italic, bold italic. They are interned first, so a face's id is
        // its style's raw value.
        interned.append(contentsOf: faces)
        for (index, font) in interned.enumerated() { ids[Self.key(font)] = UInt16(index) }

        var glyph = CGGlyph(0)
        var m: UniChar = 0x4D
        CTFontGetGlyphsForCharacters(regular, &m, &glyph, 1)
        var advance = CGSize.zero
        CTFontGetAdvancesForGlyphs(regular, .default, &glyph, &advance, 1)
        let ascent = CTFontGetAscent(regular), descent = CTFontGetDescent(regular), leading = CTFontGetLeading(regular)
        let natural = ascent + descent + leading
        let height: Int
        let baseline: Int
        if let lineHeight {
            // A set line height: the text sits in the middle of it, as CSS does.
            height = Int((size * lineHeight).rounded())
            baseline = Int(((CGFloat(height) - (ascent + descent)) / 2 + ascent).rounded())
        } else {
            height = Int(natural.rounded(.up))
            baseline = Int(ascent.rounded(.up) + (leading / 2).rounded(.down))
        }
        let thickness = max(1, Int(CTFontGetUnderlineThickness(regular).rounded()))
        let underlineBelow = max(1, Int(abs(CTFontGetUnderlinePosition(regular)).rounded()))
        let xHeight = CTFontGetXHeight(regular)
        metrics = CellMetrics(
            width: max(1, Int(advance.width.rounded())), height: max(1, height), baseline: baseline,
            underlineTop: min(height - thickness, baseline + underlineBelow - thickness / 2), lineThickness: thickness,
            strikeTop: baseline - Int((xHeight / 2).rounded()) - thickness / 2)
    }

    /// The face with the settings' OpenType features and fallback chain. Ligatures off turns `calt` and `liga`
    /// off, as the GPUI app's `ligature_features` does: programming fonts form `=>` through contextual
    /// alternates, others through standard ligatures. On, the font keeps its defaults.
    private static func configured(_ font: CTFont, settings: TerminalFontSettings, size: CGFloat) -> CTFont {
        var attributes: [CFString: Any] = [:]
        if !settings.ligatures {
            attributes[kCTFontFeatureSettingsAttribute] = ["calt", "liga"].map {
                [kCTFontOpenTypeFeatureTag: $0, kCTFontOpenTypeFeatureValue: 0] as [CFString: Any]
            }
        }
        if !settings.fallbacks.isEmpty {
            attributes[kCTFontCascadeListAttribute] = settings.fallbacks.map {
                CTFontDescriptorCreateWithNameAndSize($0 as CFString, size)
            }
        }
        guard !attributes.isEmpty else { return font }
        return CTFontCreateCopyWithAttributes(font, size, nil, CTFontDescriptorCreateWithAttributes(attributes as CFDictionary))
    }

    /// A font from a file, registered for the process so its name resolves (and fallback can find it).
    private static func load(_ url: URL, size: CGFloat) -> CTFont {
        CTFontManagerRegisterFontsForURL(url as CFURL, .process, nil)
        if let descriptors = CTFontManagerCreateFontDescriptorsFromURL(url as CFURL) as? [CTFontDescriptor], let first = descriptors.first {
            return CTFontCreateWithFontDescriptor(first, size, nil)
        }
        FileHandle.standardError.write(Data("cannot load the font \(url.path); using Menlo\n".utf8))
        return CTFontCreateWithName("Menlo" as CFString, size, nil)
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
