import Foundation
import Testing
import TerminalCore

@testable import TerminalMetal

/// Renders through the real Metal pipeline into a texture and reads pixels back: the renderer's own
/// regression checks, with no window. They need a GPU, like the app.
@MainActor
@Suite struct RendererTests {
    private let size = GridSize(cols: 20, rows: 4)

    private func renderer() throws -> MetalRenderer {
        try MetalRenderer(configuration: RendererConfiguration(font: .system(family: "Menlo"), pointSize: 13, scale: 2))
    }

    private func frame(_ cells: [[PackedCell]], cursor: FrameHeader.Cursor? = nil) -> Data {
        var writer = FrameWriter(
            header: FrameHeader(cols: size.cols, rows: cells.count, changedRows: cells.count, isFull: true, cursor: cursor),
            rowCapacity: cells.count)
        for (index, row) in cells.enumerated() {
            row.withUnsafeBufferPointer { writer.appendRow(index: index, size: .single, cells: $0) }
        }
        return writer.bytes
    }

    private func blankRows() -> [[PackedCell]] { Array(repeating: Array(repeating: PackedCell(), count: size.cols), count: size.rows) }

    @Test func aBackgroundRunIsItsColourAndTheRestIsTheDefault() throws {
        let renderer = try renderer()
        var rows = blankRows()
        for column in 2..<5 { rows[1][column].bg = PackedColor.rgb(200, 10, 20) }
        try renderer.apply(frame: frame(rows))
        let image = renderer.renderToImage()
        let cell = renderer.cellPixels
        let inside = image.pixel(3 * cell.width + cell.width / 2, 1 * cell.height + cell.height / 2)
        #expect(inside == (200, 10, 20))
        let outside = image.pixel(10 * cell.width, 2 * cell.height)
        let background = TerminalPalette.spike.background
        #expect(outside == (background.x, background.y, background.z))
    }

    @Test func aGlyphDrawsInItsForegroundAndASpaceDrawsNothing() throws {
        let renderer = try renderer()
        var rows = blankRows()
        rows[0][0] = PackedCell(scalar: 0x57, fg: PackedColor.rgb(0, 255, 0))  // W
        try renderer.apply(frame: frame(rows))
        let image = renderer.renderToImage()
        let cell = renderer.cellPixels
        var green = 0
        for y in 0..<cell.height {
            for x in 0..<cell.width where image.pixel(x, y).g > 150 && image.pixel(x, y).r < 60 { green += 1 }
        }
        #expect(green > 20, "the glyph's strokes are green")
        for y in 0..<cell.height {
            for x in cell.width..<2 * cell.width { #expect(image.pixel(x, y).g < 0x20 + 1, "the next cell is the background") }
        }
    }

    @Test func aFullBlockFillsItsCellExactly() throws {
        let renderer = try renderer()
        var rows = blankRows()
        rows[2][4] = PackedCell(scalar: 0x2588, fg: PackedColor.rgb(10, 200, 30))
        try renderer.apply(frame: frame(rows))
        let image = renderer.renderToImage()
        let cell = renderer.cellPixels
        let (x0, y0) = (4 * cell.width, 2 * cell.height)
        #expect(image.pixel(x0, y0) == (10, 200, 30), "the top-left pixel")
        #expect(image.pixel(x0 + cell.width - 1, y0 + cell.height - 1) == (10, 200, 30), "the bottom-right pixel")
        #expect(image.pixel(x0 - 1, y0) != (10, 200, 30) && image.pixel(x0 + cell.width, y0) != (10, 200, 30), "and no more")
    }

    @Test func aFilledBlockCursorTakesTheCursorColourAndAnUnfocusedOneIsAnOutline() throws {
        let renderer = try renderer()
        try renderer.apply(frame: frame(blankRows(), cursor: .init(row: 0, column: 0)))
        let cell = renderer.cellPixels
        let cursor = TerminalPalette.spike.cursor
        #expect(renderer.renderToImage().pixel(cell.width / 2, cell.height / 2) == (cursor.x, cursor.y, cursor.z))
        renderer.setFocused(false)
        let image = renderer.renderToImage()
        #expect(image.pixel(cell.width / 2, cell.height / 2) != (cursor.x, cursor.y, cursor.z), "the inside is open")
        #expect(image.pixel(cell.width / 2, 0) == (cursor.x, cursor.y, cursor.z), "the top edge is the cursor's")
    }

    @Test func onlyTheChangedRowsAreRebuilt() throws {
        let renderer = try renderer()
        let first = try renderer.apply(frame: frame(blankRows()))
        #expect(first == size.rows, "the first frame builds every row")
        var writer = FrameWriter(
            header: FrameHeader(cols: size.cols, rows: size.rows, changedRows: 1, isFull: false, cursor: nil), rowCapacity: 1)
        Array(repeating: PackedCell(scalar: 0x78), count: size.cols).withUnsafeBufferPointer {
            writer.appendRow(index: 2, size: .single, cells: $0)
        }
        #expect(try renderer.apply(frame: writer.bytes) == 1)
        let again = renderer.counters
        // Words are shaped once: the same row again is answered from the cache.
        _ = try renderer.apply(frame: writer.bytes)
        #expect((renderer.counters - again).wordsShaped == 0)
    }

    @Test func aWideCharacterAndAnEmojiDrawTwoCellsWide() throws {
        let renderer = try renderer()
        var rows = blankRows()
        rows[0][0] = PackedCell(scalar: 0x65E5, fg: PackedColor.rgb(255, 255, 255), flags: .wideChar)
        rows[0][1] = PackedCell(scalar: 0x20, flags: .wideCharSpacer)
        rows[1][0] = PackedCell(scalar: 0x1F600, flags: .wideChar)
        rows[1][1] = PackedCell(scalar: 0x20, flags: .wideCharSpacer)
        try renderer.apply(frame: frame(rows))
        let image = renderer.renderToImage()
        let cell = renderer.cellPixels
        func lit(row: Int) -> Bool {
            (0..<cell.height).contains { y in (0..<2 * cell.width).contains { x in image.pixel(x, row * cell.height + y) != image.pixel(10 * cell.width, 0) } }
        }
        #expect(lit(row: 0) && lit(row: 1))
        // The emoji is a colour bitmap: it has pixels that are not grey.
        let colourful = (0..<cell.height).contains { y in
            (0..<2 * cell.width).contains { x in
                let p = image.pixel(x, cell.height + y)
                return abs(Int(p.r) - Int(p.b)) > 60
            }
        }
        #expect(colourful)
    }
}
