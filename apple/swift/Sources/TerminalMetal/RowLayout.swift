// Where one row's parts go, in device pixels: the numbers the builder reads instead of the cell metrics, so a row
// that is double-width or half of a double-height line (DECDWL, DECDHL) differs from a normal one in these
// and nowhere else in the builder. A tall line is drawn by both of its rows, each cut to its own rectangle: the
// top row shows the upper half of the line, the one below the lower half (`render/sized.rs` says the same).

import TerminalCore

struct RowLayout {
    let stretch: Stretch
    /// A cell as drawn: twice a normal one's width on a sized row.
    let cellWidth: Float
    /// The row's own height, which is what it is cut to.
    let rowTop: Float
    let rowHeight: Float
    /// The top of the line the row draws: its own top, or the row above's for the bottom half of a tall line.
    let lineTop: Float
    let baseline: Float
    let underlineTop: Float
    let strikeTop: Float
    /// The lines through and under the text are as thick as the text is tall.
    let thickness: Float
    /// How many columns the row shows (half, on a sized row), and where the grid's right edge is.
    let shown: Int
    let gridRight: Float

    init(row: Int, size: LineSize, columns: Int, metrics: CellMetrics, stretched: CellMetrics) {
        stretch = Stretch(size)
        cellWidth = Float(stretched.width)
        rowHeight = Float(metrics.height)
        rowTop = Float(row * metrics.height)
        lineTop = size == .doubleHeightBottom ? rowTop - rowHeight : rowTop
        baseline = lineTop + Float(stretched.baseline)
        underlineTop = lineTop + Float(stretched.underlineTop)
        strikeTop = lineTop + Float(stretched.strikeTop)
        thickness = Float(stretched.lineThickness)
        shown = stretch == .normal ? columns : (columns + 1) / 2
        gridRight = Float(columns * metrics.width)
    }

    var isTall: Bool { stretch == .tall }

    /// A solid quad cut to the row; `nil` if nothing of it is inside. Only a tall line reaches past its rows.
    func clipped(_ quad: SolidInstance) -> SolidInstance? {
        guard isTall else { return quad }
        let top = max(quad.y, rowTop), bottom = min(quad.y + quad.height, rowTop + rowHeight)
        guard bottom > top else { return nil }
        var cut = quad
        cut.y = top
        cut.height = bottom - top
        return cut
    }

    /// A glyph cut to the row, its atlas rectangle cut the same (the pixels map one to one).
    func clipped(_ glyph: GlyphInstance) -> GlyphInstance? {
        guard isTall else { return glyph }
        let top = max(glyph.y, rowTop), bottom = min(glyph.y + glyph.height, rowTop + rowHeight)
        guard bottom > top else { return nil }
        var cut = glyph
        let above = Int(top - glyph.y)
        cut.y = top
        cut.height = bottom - top
        cut.atlasY = glyph.atlasY + UInt16(above)
        cut.atlasHeight = UInt16(bottom - top)
        return cut
    }
}
