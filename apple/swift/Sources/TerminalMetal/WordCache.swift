// The word cache: text shaped once, painted from then on. A "word" is a run of single-width cells in one
// font face; CoreText shapes it on a miss and the result (which glyph sits in which column, and where its
// bitmap is in the atlas) is kept under a hash of the text and the face. Colours are not in the key: a word
// is painted glyph by glyph in the colours of the moment, as GPUI's cache does.
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
    var atlas: Int32
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

    private let fonts: FontSet
    private let atlas: GlyphAtlas
    private var table: [UInt64: Record] = [:]
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
    static func hash(style: FontStyle, stretch: Stretch = .normal, scalars: UnsafePointer<UInt32>, count: Int) -> UInt64 {
        var h: UInt64 = 0xcbf2_9ce4_8422_2325 ^ UInt64(style.rawValue) ^ stretch.rawValue << 8
        for index in 0..<count { h = (h ^ UInt64(scalars[index])) &* 0x100_0000_01b3 }
        return h
    }

    /// The glyphs of the word `scalars[0..<count]` in `style`: a range into `glyphs`.
    func glyphRange(style: FontStyle, stretch: Stretch = .normal, scalars text: UnsafePointer<UInt32>, count: Int, hash: UInt64) -> (start: Int, count: Int) {
        if let record = table[hash], Int(record.scalarCount) == count,
            memcmp(scalars.base + Int(record.scalarStart), text, count * MemoryLayout<UInt32>.size) == 0
        {
            hits += 1
            return (Int(record.glyphStart), Int(record.glyphCount))
        }
        if table.count >= Self.wordLimit { clear() }
        shaped += 1
        let buffer = UnsafeBufferPointer(start: text, count: count)
        var shapedGlyphs = shape(buffer, style: style, stretch: stretch)
        // If the atlas started over while this word was being shaped, its first glyphs point at entries that
        // are gone: once more, now that the atlas is empty.
        if shapedGlyphs.generation != atlas.generation { shapedGlyphs = shape(buffer, style: style, stretch: stretch) }
        // `shape` may have started the atlas (and so this cache) over; the arenas are consistent either way.
        let record = Record(
            scalarStart: Int32(scalars.count), scalarCount: Int32(count), glyphStart: Int32(glyphs.count),
            glyphCount: Int32(shapedGlyphs.glyphs.count))
        for index in 0..<count { scalars.append(text[index]) }
        for glyph in shapedGlyphs.glyphs { glyphs.append(glyph) }
        table[hash] = record
        return (Int(record.glyphStart), Int(record.glyphCount))
    }

    /// The atlas entry of an exact box-drawing, block or braille cell, made for the cell of a row of `stretch`.
    func boxEntry(_ scalar: UInt32, stretch: Stretch = .normal) -> Int32 {
        let key = stretch.boxKey(scalar)
        if let id = atlas.lookup(key) { return id }
        let raster = BoxDrawing.raster(scalar, metrics: fonts.metrics.stretched(stretch))
        return add(raster, key: key)
    }

    func clear() {
        table.removeAll(keepingCapacity: true)
        scalars.removeAll()
        glyphs.removeAll()
        clears += 1
    }

    // MARK: shaping

    private func shape(_ text: UnsafeBufferPointer<UInt32>, style: FontStyle, stretch: Stretch) -> (glyphs: [WordGlyph], generation: Int) {
        let generation = atlas.generation
        var string = String.UnicodeScalarView()
        /// For each UTF-16 unit of the string, the cell it came from.
        var columns: [UInt16] = []
        columns.reserveCapacity(text.count)
        for (column, value) in text.enumerated() {
            let scalar = Unicode.Scalar(value) ?? "\u{FFFD}"
            string.append(scalar)
            for _ in 0..<scalar.utf16.count { columns.append(UInt16(column)) }
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
            for index in 0..<count where ids[index] != 0 {
                let key = stretch.key(font: fontID, glyph: ids[index])
                let entry = atlas.lookup(key) ?? add(GlyphRaster.rasterize(font: font, glyph: ids[index], stretch: stretch), key: key)
                let column = columns[min(indices[index], columns.count - 1)]
                result.append(WordGlyph(column: column, atlas: entry))
            }
        }
        return (result, generation)
    }

    /// Add a raster, starting the atlas and this cache over if the texture is full.
    private func add(_ raster: Raster, key: UInt64) -> Int32 {
        if let id = atlas.add(raster, key: key) { return id }
        atlas.reset()
        clear()
        // A glyph bigger than an empty texture cannot be stored; draw nothing for it rather than loop.
        return atlas.add(raster, key: key) ?? atlas.add(.empty, key: key) ?? 0
    }
}
