// A row of packed cells into instances: background runs, glyphs from the word cache, and decorations.
// This is the frame loop's hot path, so it reads cells through an unsafe buffer, keeps its state in
// locals and appends into arrays that keep their capacity between frames.

import CoreGraphics
import Foundation
import TerminalCore

/// How the cursor is drawn in a row that holds it.
struct CursorDraw {
    var column: Int
    var shape: CursorShape
    var wide: Bool
    /// The window is key: a block is filled; otherwise it is an outline.
    var focused: Bool
}

final class RowBuilder {
    private let metrics: CellMetrics
    private let atlas: GlyphAtlas
    private let words: WordCache
    private let scratch = UnsafeMutablePointer<UInt32>.allocate(capacity: WordCache.maxWordCells)

    init(fonts: FontSet, atlas: GlyphAtlas, words: WordCache) {
        metrics = fonts.metrics
        self.atlas = atlas
        self.words = words
    }

    deinit { scratch.deallocate() }

    /// A cell's colours as drawn: inverse swaps them, hidden paints the text in the background, and a
    /// filled block cursor takes the cursor's colour with the cell's background as the text.
    @inline(__always)
    private func colors(_ cell: PackedCell, _ flags: CellFlags, underBlockCursor: Bool) -> (fg: UInt32, bg: UInt32) {
        var fg = cell.fg, bg = cell.bg
        if flags.contains(.inverse) { swap(&fg, &bg) }
        if underBlockCursor {
            fg = bg
            bg = PackedColor.cursor
        }
        if flags.contains(.hidden) { fg = bg }
        return (fg, bg)
    }

    func build(row: Int, cells: UnsafeBufferPointer<PackedCell>, into geometry: RowGeometry, cursor: CursorDraw?) {
        geometry.removeAll()
        let cellWidth = Float(metrics.width), cellHeight = Float(metrics.height)
        let top = Float(row * metrics.height)
        let baseline = top + Float(metrics.baseline)
        let thickness = Float(metrics.lineThickness)
        let columns = cells.count

        let filledBlock = cursor.map { $0.shape == .block && $0.focused } ?? false
        func covered(_ column: Int) -> Bool {
            guard filledBlock, let cursor else { return false }
            return column == cursor.column || (cursor.wide && column == cursor.column + 1)
        }

        // Open runs, each flushed when something different arrives or the row ends.
        var backgroundColor = PackedColor.background, backgroundStart = -1
        var decoration: UInt32 = 0, decorationColor: UInt32 = 0, decorationStart = -1
        var strikeColor: UInt32 = 0, strikeStart = -1
        var overColor: UInt32 = 0, overStart = -1
        var wordStyle = FontStyle.regular, wordStart = -1, wordLength = 0

        func flushBackground(_ end: Int) {
            guard backgroundStart >= 0 else { return }
            geometry.backgrounds.append(
                SolidInstance(
                    x: Float(backgroundStart) * cellWidth, y: top, width: Float(end - backgroundStart) * cellWidth,
                    height: cellHeight, color: backgroundColor, style: 0))
            backgroundStart = -1
        }
        func flushDecoration(_ end: Int) {
            guard decorationStart >= 0 else { return }
            let x = Float(decorationStart) * cellWidth, width = Float(end - decorationStart) * cellWidth
            let underline = top + Float(metrics.underlineTop)
            switch CellFlags(rawValue: decoration) {
            case .doubleUnderline:
                // Two lines, kept inside the cell: the second goes below only if there is room.
                let gap = thickness * 2
                let first = underline + gap + thickness > top + cellHeight ? underline - gap : underline
                geometry.decorations.append(SolidInstance(x: x, y: first, width: width, height: thickness, color: decorationColor, style: 0))
                geometry.decorations.append(SolidInstance(x: x, y: first + gap, width: width, height: thickness, color: decorationColor, style: 0))
            case .undercurl:
                let height = thickness * 3
                let y = min(underline - thickness, top + cellHeight - height)
                geometry.decorations.append(SolidInstance(x: x, y: y, width: width, height: height, color: decorationColor, style: SolidInstance.curly))
            case .dottedUnderline:
                geometry.decorations.append(SolidInstance(x: x, y: underline, width: width, height: thickness, color: decorationColor, style: SolidInstance.dotted))
            case .dashedUnderline:
                geometry.decorations.append(SolidInstance(x: x, y: underline, width: width, height: thickness, color: decorationColor, style: SolidInstance.dashed))
            default:
                geometry.decorations.append(SolidInstance(x: x, y: underline, width: width, height: thickness, color: decorationColor, style: 0))
            }
            decorationStart = -1
        }
        func flushStrike(_ end: Int) {
            guard strikeStart >= 0 else { return }
            geometry.decorations.append(
                SolidInstance(
                    x: Float(strikeStart) * cellWidth, y: top + Float(metrics.strikeTop),
                    width: Float(end - strikeStart) * cellWidth, height: thickness, color: strikeColor, style: 0))
            strikeStart = -1
        }
        func flushOver(_ end: Int) {
            guard overStart >= 0 else { return }
            geometry.decorations.append(
                SolidInstance(
                    x: Float(overStart) * cellWidth, y: top, width: Float(end - overStart) * cellWidth, height: thickness,
                    color: overColor, style: 0))
            overStart = -1
        }
        func emit(_ id: Int32, column: Int, cell: PackedCell) {
            let entry = atlas.entry(id)
            guard !entry.isEmpty else { return }
            let flags = CellFlags(rawValue: cell.flags)
            let (fg, _) = colors(cell, flags, underBlockCursor: covered(column))
            geometry.glyphs.append(
                GlyphInstance(
                    x: Float(column) * cellWidth + Float(entry.left), y: baseline - Float(entry.top),
                    width: Float(entry.width), height: Float(entry.height), atlasX: entry.x, atlasY: entry.y,
                    atlasWidth: entry.width, atlasHeight: entry.height, color: fg,
                    flags: (flags.contains(.dim) ? GlyphInstance.dimBit : 0) | (entry.isColor ? GlyphInstance.colorBit : 0)))
        }
        func flushWord(_ end: Int) {
            guard wordLength > 0 else { return }
            let hash = WordCache.hash(style: wordStyle, scalars: scratch, count: wordLength)
            let range = words.glyphRange(style: wordStyle, scalars: scratch, count: wordLength, hash: hash)
            let table = words.glyphs.base
            for index in range.start..<range.start + range.count {
                let glyph = table[index]
                let column = wordStart + Int(glyph.column)
                emit(glyph.atlas, column: column, cell: cells[column])
            }
            wordLength = 0
            wordStart = -1
        }

        for column in 0..<columns {
            let cell = cells[column]
            let flags = CellFlags(rawValue: cell.flags)
            let (fg, bg) = colors(cell, flags, underBlockCursor: covered(column))

            // Background: runs of one colour; the default needs no quad (the pass clears to it).
            if bg != PackedColor.background {
                if backgroundStart >= 0, bg == backgroundColor {
                } else {
                    flushBackground(column)
                    backgroundStart = column
                    backgroundColor = bg
                }
            } else {
                flushBackground(column)
            }

            // Decorations: a run ends when the style or the colour changes.
            let underline = flags.intersection(.allUnderlines).rawValue
            if underline != 0 {
                if decorationStart >= 0, decoration == underline, decorationColor == fg {
                } else {
                    flushDecoration(column)
                    decorationStart = column
                    decoration = underline
                    decorationColor = fg
                }
            } else {
                flushDecoration(column)
            }
            if flags.contains(.strikeout) {
                if strikeStart >= 0, strikeColor == fg {
                } else {
                    flushStrike(column)
                    strikeStart = column
                    strikeColor = fg
                }
            } else {
                flushStrike(column)
            }
            if flags.contains(.overline) {
                if overStart >= 0, overColor == fg {
                } else {
                    flushOver(column)
                    overStart = column
                    overColor = fg
                }
            } else {
                flushOver(column)
            }

            // Text.
            let scalar = cell.scalar
            if flags.contains(.wideCharSpacer) || flags.contains(.leadingWideCharSpacer) {
                flushWord(column)
            } else if scalar <= 0x20 || scalar == 0xA0 {
                flushWord(column)
            } else if flags.contains(.wideChar) {
                // A double-width character stands alone, looked up as a word of one.
                flushWord(column)
                scratch[0] = scalar
                let style = FontStyle(bold: flags.contains(.bold), italic: flags.contains(.italic))
                let hash = WordCache.hash(style: style, scalars: scratch, count: 1)
                let range = words.glyphRange(style: style, scalars: scratch, count: 1, hash: hash)
                for index in range.start..<range.start + range.count { emit(words.glyphs.base[index].atlas, column: column, cell: cell) }
            } else if BoxDrawing.handles(scalar) {
                flushWord(column)
                emit(words.boxEntry(scalar), column: column, cell: cell)
            } else {
                let style = FontStyle(bold: flags.contains(.bold), italic: flags.contains(.italic))
                if wordLength > 0, style != wordStyle || wordLength == WordCache.maxWordCells { flushWord(column) }
                if wordLength == 0 {
                    wordStart = column
                    wordStyle = style
                }
                scratch[wordLength] = scalar
                wordLength += 1
            }
        }
        flushWord(columns)
        flushBackground(columns)
        flushDecoration(columns)
        flushStrike(columns)
        flushOver(columns)

        if let cursor, !filledBlock { addCursorShape(cursor, top: top, geometry: geometry) }
    }

    /// The cursor shapes that are not a filled block, as decoration quads over the glyphs.
    private func addCursorShape(_ cursor: CursorDraw, top: Float, geometry: RowGeometry) {
        let cellWidth = Float(metrics.width), cellHeight = Float(metrics.height)
        let x = Float(cursor.column) * cellWidth
        let width = cellWidth * (cursor.wide ? 2 : 1)
        let thickness = Float(max(1, metrics.lineThickness))
        let color = PackedColor.cursor
        func quad(_ x: Float, _ y: Float, _ w: Float, _ h: Float) {
            geometry.decorations.append(SolidInstance(x: x, y: y, width: w, height: h, color: color, style: 0))
        }
        switch cursor.shape {
        case .beam: quad(x, top, thickness, cellHeight)
        case .underline: quad(x, top + cellHeight - thickness, width, thickness)
        case .block, .hollow:
            // Not filled (not focused, or asked for): an outline, so the text under it stays readable.
            quad(x, top, width, thickness)
            quad(x, top + cellHeight - thickness, width, thickness)
            quad(x, top + thickness, thickness, cellHeight - 2 * thickness)
            quad(x + width - thickness, top + thickness, thickness, cellHeight - 2 * thickness)
        }
    }
}
