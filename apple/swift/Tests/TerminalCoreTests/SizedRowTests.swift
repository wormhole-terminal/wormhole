import Foundation
import Testing
import TerminalCore

@testable import TerminalMetal

/// DECDWL and DECDHL: a sized row shows half its columns, each twice as wide, and a tall line is drawn by both
/// of its rows, each showing its half. Drawn through the real pipeline and read back.
@MainActor
@Suite struct SizedRowTests {
    private let block = PackedCell(scalar: 0x2588, fg: PackedColor.rgb(10, 200, 30))
    private let green: (UInt8, UInt8, UInt8) = (10, 200, 30)

    @Test func aDoubleWidthRowDrawsTwiceAsWideCellsAndShowsOnlyTheFirstHalf() throws {
        let renderer = try RowDrawing.renderer()
        var cells = RowDrawing.blank()
        cells[1][3] = block
        cells[1][12] = block  // past the half (10 of 20): hidden, as in the engine
        try renderer.apply(frame: RowDrawing.frame(cells, sizes: [.single, .doubleWidth]))
        let image = renderer.renderToImage()
        let cell = renderer.cellPixels
        let mid = cell.height + cell.height / 2
        #expect(image.pixel(6 * cell.width + 2, mid) == green, "column 3 now starts at 6 cells")
        #expect(image.pixel(8 * cell.width - 2, mid) == green, "and is two cells wide")
        #expect(image.pixel(3 * cell.width + 2, mid) != green, "not where an ordinary cell 3 would be")
        #expect(!RowDrawing.lit(image, x: 12 * cell.width..<20 * cell.width, y: cell.height..<2 * cell.height, background: RowDrawing.background))
    }

    @Test func aDoubleHeightLineIsDrawnByBothRowsEachShowingItsHalf() throws {
        let renderer = try RowDrawing.renderer()
        var cells = RowDrawing.blank()
        let upperHalf = PackedCell(scalar: 0x2580, fg: PackedColor.rgb(10, 200, 30))  // ▀
        cells[0][0] = upperHalf
        cells[1][0] = upperHalf
        try renderer.apply(frame: RowDrawing.frame(cells, sizes: [.doubleHeightTop, .doubleHeightBottom]))
        let image = renderer.renderToImage()
        let cell = renderer.cellPixels
        #expect(image.pixel(cell.width, cell.height / 2) == green, "the upper half of a tall cell is the whole top row")
        #expect(image.pixel(cell.width, cell.height - 1) == green)
        #expect(!RowDrawing.lit(image, x: 0..<2 * cell.width, y: cell.height..<2 * cell.height, background: RowDrawing.background),
            "and nothing of the row below")
    }

    @Test func theLowerRowOfATallLineShowsTheLowerHalf() throws {
        let renderer = try RowDrawing.renderer()
        var cells = RowDrawing.blank()
        let lowerHalf = PackedCell(scalar: 0x2584, fg: PackedColor.rgb(10, 200, 30))  // ▄
        cells[0][0] = lowerHalf
        cells[1][0] = lowerHalf
        try renderer.apply(frame: RowDrawing.frame(cells, sizes: [.doubleHeightTop, .doubleHeightBottom]))
        let image = renderer.renderToImage()
        let cell = renderer.cellPixels
        #expect(!RowDrawing.lit(image, x: 0..<2 * cell.width, y: 0..<cell.height, background: RowDrawing.background))
        #expect(image.pixel(cell.width, cell.height + cell.height / 2) == green)
    }

    @Test func glyphsOfASizedRowAreRasterisedAtTheirSizeNotScaled() throws {
        let renderer = try RowDrawing.renderer()
        var cells = RowDrawing.blank()
        cells[0][0] = PackedCell(scalar: 0x57)  // W
        cells[1][0] = PackedCell(scalar: 0x57)
        try renderer.apply(frame: RowDrawing.frame(cells, sizes: [.single, .doubleWidth]))
        #expect(renderer.counters.glyphsRasterised == 2, "one bitmap for the normal W, one for the stretched one")
    }

    @Test func underlinesOnATallLineAreAsThickAsItsTextIsTall() throws {
        let plain = try RowDrawing.renderer(), tall = try RowDrawing.renderer()
        var cells = RowDrawing.blank()
        cells[0][0] = PackedCell(scalar: 0x20, flags: .underline)
        cells[1][0] = PackedCell(scalar: 0x20, flags: .underline)
        try plain.apply(frame: RowDrawing.frame(cells))
        try tall.apply(frame: RowDrawing.frame(cells, sizes: [.doubleHeightTop, .doubleHeightBottom]))
        // One line's own underline: the plain line has one in each row, the tall line one for both.
        func thickness(_ renderer: MetalRenderer, rows: Int) -> Int {
            let image = renderer.renderToImage(), cell = renderer.cellPixels
            return (0..<rows * cell.height).filter { image.pixel(1, $0) != RowDrawing.background }.count
        }
        #expect(thickness(plain, rows: 1) == plain.scene.fonts.metrics.lineThickness)
        #expect(thickness(tall, rows: 2) == 2 * plain.scene.fonts.metrics.lineThickness)
    }
}
