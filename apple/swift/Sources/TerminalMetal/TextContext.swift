// Everything that is made for one font at one size and scale: the faces and the cell they imply, the glyph atlas,
// the word cache and the row builder that reads them. A font change builds (or finds, `TextContexts`) another one
// and the scene starts drawing with it; none of it is edited in place, so frames still in flight on the GPU keep
// the textures they were encoded with.

import Metal

final class TextContext {
    let fonts: FontSet
    let atlas: GlyphAtlas
    let words: WordCache
    let builder: RowBuilder

    init(settings: TerminalFontSettings, scale: Double, device: MTLDevice) {
        fonts = FontSet(settings: settings, scale: CGFloat(scale))
        atlas = GlyphAtlas(device: device)
        words = WordCache(fonts: fonts, atlas: atlas)
        builder = RowBuilder(fonts: fonts, atlas: atlas, words: words)
    }
}

/// What the work counters of the renderer count: words shaped, glyphs rasterised and the like, over every
/// context the renderer has had, so a font change does not reset what the measuring mode reads.
struct TextTotals: Equatable {
    var wordsShaped = 0
    var wordHits = 0
    var glyphsRasterised = 0
    var atlasResets = 0

    init() {}

    init(_ context: TextContext) {
        wordsShaped = context.words.shaped
        wordHits = context.words.hits
        glyphsRasterised = context.atlas.rasterised
        atlasResets = context.atlas.resets
    }

    static func + (lhs: Self, rhs: Self) -> Self {
        var sum = lhs
        sum.wordsShaped += rhs.wordsShaped
        sum.wordHits += rhs.wordHits
        sum.glyphsRasterised += rhs.glyphsRasterised
        sum.atlasResets += rhs.atlasResets
        return sum
    }
}

/// The few contexts most recently used. A text zoom passes through a handful of quarter-point sizes and comes
/// back to some of them (a second press, ⌘0); a settings change back to a previous font finds its context with
/// the glyphs already rasterised. The textures are 8 MB a context, so the cache is short.
final class TextContexts {
    private struct Key: Hashable {
        var settings: TerminalFontSettings
        var scale: Double
    }

    private static let capacity = 4
    private let device: MTLDevice
    private var recent: [(key: Key, context: TextContext)] = []
    private var retired = TextTotals()

    init(device: MTLDevice) { self.device = device }

    func context(for settings: TerminalFontSettings, scale: Double) -> TextContext {
        let key = Key(settings: settings, scale: scale)
        if let index = recent.firstIndex(where: { $0.key == key }) {
            let entry = recent.remove(at: index)
            recent.append(entry)
            return entry.context
        }
        let context = TextContext(settings: settings, scale: scale, device: device)
        recent.append((key, context))
        if recent.count > Self.capacity { retired = retired + TextTotals(recent.removeFirst().context) }
        return context
    }

    /// The counters of every context there has been.
    var totals: TextTotals { recent.reduce(retired) { $0 + TextTotals($1.context) } }
}
