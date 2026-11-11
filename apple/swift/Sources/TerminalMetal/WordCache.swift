// The word cache: text shaped once, painted from then on. A "word" is a run of single-width cells in one
// font face; CoreText shapes it on a miss and the result (which glyph sits in which column) is kept under a hash
// of the text and the face. Colours are not in the key: a word is painted glyph by glyph in the colours of the
// moment, as GPUI's cache does. A glyph's bitmap depends on where between pixels its cell starts and on its colour's
// stem dilation (`GlyphRaster`), so a shaped glyph names a slot: the glyph of a face, with an atlas entry per
// quarter-pixel variant and dilation, each rasterised the first time a row needs it.
//
// Glyphs are pinned to their cell's column, never placed at CoreText's advances, so a fallback font with
// a different advance cannot shift the grid. Ligatures are the font's own contextual forms (`calt`, `liga`),
// on unless the font set was made without them (`TerminalFontSettings.ligatures`): a ligature is a glyph per
// cell that overhangs its neighbours, or one glyph in the first cell and nothing in the rest, and both draw
// right from the pinned columns. A word never holds a blank, so no ligature reaches across one.

import CoreGraphics
import CoreText
import Foundation

/// One glyph of a cached word.
struct WordGlyph {
    /// The column, counted from the word's first cell.
    var column: UInt16
    /// The glyph's slot in the cache (`WordCache.entry`).
    var slot: Int32
}

/// Plain-old-data storage that grows and never shrinks until cleared: the word cache's arenas, read in the
/// frame loop without retain or bounds checks.
final class Arena<Element> {
    private(set) var base: UnsafeMutablePointer<Element>
    private(set) var count = 0
    private var capacity: Int

    init(capacity: Int = 1024) {
        self.capacity = capacity
        base = .allocate(capacity: capacity)
    }

    deinit { base.deallocate() }

    func append(_ element: Element) {
        if count == capacity { grow() }
        (base + count).initialize(to: element)
        count += 1
    }

    func removeAll() { count = 0 }

    private func grow() {
        let bigger = UnsafeMutablePointer<Element>.allocate(capacity: capacity * 2)
        bigger.moveInitialize(from: base, count: count)
        base.deallocate()
        base = bigger
        capacity *= 2
    }
}

final class WordCache {
    /// The most cells shaped as one word; longer runs are split (GPUI's `MAX_SHAPED_CELLS`).
    static let maxWordCells = 64
    /// Words kept before the cache starts again (a flood of unique words must not grow it without end).
    private static let wordLimit = 200_000

    private struct Record {
        var scalarStart: Int32
        var scalarCount: Int32
        var glyphStart: Int32
        var glyphCount: Int32
    }

    /// A glyph of a face in a stretch, with an atlas entry per variant and dilation.
    private struct Slot {
        var font: UInt16
        var glyph: CGGlyph
        var stretch: Stretch
        var isColor: Bool
    }

    /// Atlas entries kept per slot: `GlyphRaster.variants` x five dilations.
    static let entriesPerSlot = GlyphRaster.variants * 5

    private let fonts: FontSet
    private let atlas: GlyphAtlas
    private var table: [UInt64: Record] = [:]
    private var slots = ContiguousArray<Slot>()
    private var slotIndex: [UInt64: Int32] = [:]
    /// `entriesPerSlot` atlas ids per slot, -1 until rasterised; all -1 again when the atlas starts over.
    private var entries = ContiguousArray<Int32>()
    /// Box-drawing keys the core leaves to the font.
    private var fontDrawn = Set<UInt64>()
    private let scalars = Arena<UInt32>(capacity: 8192)
    let glyphs = Arena<WordGlyph>(capacity: 8192)

    /// Words shaped by CoreText, lookups answered from the cache, and starts from empty.
    private(set) var shaped = 0
    private(set) var hits = 0
    private(set) var clears = 0

    init(fonts: FontSet, atlas: GlyphAtlas) {
        self.fonts = fonts
        self.atlas = atlas
    }

    @inline(__always)
    static func hash(style: FontStyle, stretch: Stretch = .normal, cluster: Bool = false, scalars: UnsafePointer<UInt32>, count: Int) -> UInt64 {
        var h: UInt64 = 0xcbf2_9ce4_8422_2325 ^ UInt64(style.rawValue) ^ stretch.rawValue << 8 ^ (cluster ? 1 << 16 : 0)
        for index in 0..<count { h = (h ^ UInt64(scalars[index])) &* 0x100_0000_01b3 }
        return h
    }

    /// The glyphs of the word `scalars[0..<count]` in `style`: a range into `glyphs`. A `cluster` is one cell's character
    /// followed by its zero-width marks, shaped together.
    func glyphRange(style: FontStyle, stretch: Stretch = .normal, cluster: Bool = false, scalars text: UnsafePointer<UInt32>, count: Int, hash: UInt64) -> (start: Int, count: Int) {
        if let record = table[hash], Int(record.scalarCount) == count,
            memcmp(scalars.base + Int(record.scalarStart), text, count * MemoryLayout<UInt32>.size) == 0
        {
            hits += 1
            return (Int(record.glyphStart), Int(record.glyphCount))
        }
        if table.count >= Self.wordLimit { clear() }
        shaped += 1
        let buffer = UnsafeBufferPointer(start: text, count: count)
        let shapedGlyphs = shape(buffer, style: style, stretch: stretch, cluster: cluster)
        let record = Record(
            scalarStart: Int32(scalars.count), scalarCount: Int32(count), glyphStart: Int32(glyphs.count),
            glyphCount: Int32(shapedGlyphs.count))
        for index in 0..<count { scalars.append(text[index]) }
        for glyph in shapedGlyphs { glyphs.append(glyph) }
        table[hash] = record
        return (Int(record.glyphStart), Int(record.glyphCount))
    }

    /// Whether a slot's glyph is a colour one (drawn at a whole pixel, as GPUI's emoji are).
    @inline(__always)
    func isColor(_ slot: Int32) -> Bool { slots[Int(slot)].isColor }

    /// The atlas entry of a slot's glyph at a quarter-pixel `variant` and a `dilation`, rasterised on first use.
    @inline(__always)
    func entry(_ slot: Int32, variant: Int, dilation: Int) -> Int32 {
        let index = Int(slot) * Self.entriesPerSlot + variant * 5 + dilation
        let id = entries[index]
        if id >= 0 { return id }
        let info = slots[Int(slot)]
        let font = fonts.font(id: info.font)
        let raster = GlyphRaster.rasterize(
            font: font, glyph: info.glyph, scale: fonts.scale, stretch: info.stretch, variant: variant, dilation: dilation)
        let key = info.stretch.key(font: info.font, glyph: info.glyph, variant: variant, dilation: dilation)
        let added = add(raster, key: key)
        entries[index] = added
        return added
    }

    /// The atlas entry of a box-drawing or block cell `width` whole pixels wide whose fractional cell starts `phase` past
    /// its left pixel, in a row of `stretch`; nil when the font draws the character.
    func boxEntry(_ scalar: UInt32, stretch: Stretch = .normal, width: Int, phase: Double) -> Int32? {
        let key = stretch.boxKey(scalar, width: width, phase: Int((phase * 1024).rounded()))
        if let id = atlas.lookup(key) { return id }
        if fontDrawn.contains(key) { return nil }
        let metrics = fonts.metrics.stretched(stretch)
        guard let raster = BoxDrawing.raster(
            scalar, width: width, metrics: metrics, cellWidth: metrics.width, phase: phase, fontSize: Double(fonts.pointSize),
            scale: Double(fonts.scale))
        else {
            fontDrawn.insert(key)
            return nil
        }
        return add(raster, key: key)
    }

    func clear() {
        table.removeAll(keepingCapacity: true)
        scalars.removeAll()
        glyphs.removeAll()
        clears += 1
    }

    // MARK: shaping

    private func shape(_ text: UnsafeBufferPointer<UInt32>, style: FontStyle, stretch: Stretch, cluster: Bool) -> [WordGlyph] {
        var string = String.UnicodeScalarView()
        /// For each UTF-16 unit of the string, the cell it came from.
        var columns: [UInt16] = []
        columns.reserveCapacity(text.count)
        for (column, value) in text.enumerated() {
            let scalar = Unicode.Scalar(value) ?? "\u{FFFD}"
            string.append(scalar)
            // A cluster is one cell's character and its marks: every glyph belongs to that cell.
            for _ in 0..<scalar.utf16.count { columns.append(cluster ? 0 : UInt16(column)) }
        }
        // The font carries the features (`FontSet.configured`): CoreText shapes with them.
        let attributes: [CFString: Any] = [kCTFontAttributeName: fonts.font(for: style)]
        let attributed = CFAttributedStringCreate(nil, String(string) as CFString, attributes as CFDictionary)!
        let line = CTLineCreateWithAttributedString(attributed)
        var result: [WordGlyph] = []
        let runs = CTLineGetGlyphRuns(line) as! [CTRun]
        for run in runs {
            let count = CTRunGetGlyphCount(run)
            guard count > 0 else { continue }
            var ids = [CGGlyph](repeating: 0, count: count)
            var indices = [CFIndex](repeating: 0, count: count)
            CTRunGetGlyphs(run, CFRange(location: 0, length: 0), &ids)
            CTRunGetStringIndices(run, CFRange(location: 0, length: 0), &indices)
            let attributes = CTRunGetAttributes(run) as NSDictionary
            let font = (attributes[kCTFontAttributeName] as! CTFont?) ?? fonts.font(for: style)
            let fontID = fonts.id(of: font)
            let color = FontSet.hasColorGlyphs(font)
            for index in 0..<count where ids[index] != 0 {
                let column = columns[min(indices[index], columns.count - 1)]
                result.append(WordGlyph(column: column, slot: slot(font: fontID, glyph: ids[index], stretch: stretch, isColor: color)))
            }
        }
        return result
    }

    private func slot(font: UInt16, glyph: CGGlyph, stretch: Stretch, isColor: Bool) -> Int32 {
        let key = stretch.key(font: font, glyph: glyph, variant: 0, dilation: 0)
        if let slot = slotIndex[key] { return slot }
        let slot = Int32(slots.count)
        slots.append(Slot(font: font, glyph: glyph, stretch: stretch, isColor: isColor))
        entries.append(contentsOf: repeatElement(-1, count: Self.entriesPerSlot))
        slotIndex[key] = slot
        return slot
    }

    /// Add a raster, starting the atlas over if the texture is full (every slot's entries are then made again as rows
    /// ask for them; the scene rebuilds its rows when the atlas's generation changes).
    private func add(_ raster: Raster, key: UInt64) -> Int32 {
        if let id = atlas.add(raster, key: key) { return id }
        atlas.reset()
        for index in entries.indices { entries[index] = -1 }
        // A glyph bigger than an empty texture cannot be stored; draw nothing for it rather than loop.
        return atlas.add(raster, key: key) ?? atlas.add(.empty, key: key) ?? 0
    }
}
