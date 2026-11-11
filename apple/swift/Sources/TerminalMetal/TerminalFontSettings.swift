// What the terminal's font is made of: the settings Settings → Fonts holds for it (`font_family`, `font_size`,
// `line_height`, `font_ligatures`; `crates/wormhole-core/src/config/settings`), plus the fallback families a
// glyph the family lacks is looked up in before CoreText's own cascade. A value: the host builds one from the
// settings and hands it to `MetalTerminalView.setFonts`, which rebuilds only what the change touches.

import CoreGraphics
import TerminalCore

public struct TerminalFontSettings: Sendable, Hashable {
    public var font: FontSpec
    /// In points, before the window's zoom.
    public var pointSize: Double
    /// A multiple of the font size (Settings keeps it within 1...2); `nil` takes the font's own line.
    public var lineHeight: Double?
    /// Combine `=>`, `!=` and the like into one glyph, in fonts that have them (GPUI's default is on).
    public var ligatures: Bool
    /// Families tried, in this order, for a character the font has no glyph for.
    public var fallbacks: [String]
    /// Moves when font files were registered or dropped (the fonts folder): a family that was missing and drew in a fallback
    /// face is looked up again, which equal settings would not do, and the contexts made before it are not found.
    public var folderRevision: Int

    public init(
        font: FontSpec = .system(family: "Menlo"), pointSize: Double = 13, lineHeight: Double? = nil, ligatures: Bool = true,  // text: a font family's name
        fallbacks: [String] = [], folderRevision: Int = 0
    ) {
        self.folderRevision = folderRevision
        self.font = font
        self.pointSize = pointSize
        self.lineHeight = lineHeight
        self.ligatures = ligatures
        self.fallbacks = fallbacks
    }

    /// The same fonts at another size, kept within what Settings and the zoom allow (8...36).
    func at(pointSize size: Double) -> Self {
        var copy = self
        copy.pointSize = min(max(size, TextZoom.sizes.lowerBound), TextZoom.sizes.upperBound)
        return copy
    }
}
