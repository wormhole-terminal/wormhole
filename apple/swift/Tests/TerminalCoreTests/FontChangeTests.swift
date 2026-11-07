import Foundation
import Testing
import TerminalCore

@testable import TerminalMetal

@MainActor
@Suite struct FontChangeTests {
    private func rows(_ text: String) -> [[PackedCell]] {
        var rows = RowDrawing.blank()
        for (column, scalar) in text.unicodeScalars.enumerated() { rows[0][column] = PackedCell(scalar: scalar.value) }
        return rows
    }

    @Test func aFontChangeKeepsTheGridAndTheTextAndDrawsAtTheNewCellSize() throws {
        let renderer = try RowDrawing.renderer()
        try renderer.apply(frame: RowDrawing.frame(rows("hello")))
        let before = renderer.cellPixels
        renderer.setPointSize(26)
        let after = renderer.cellPixels
        #expect(after.width > before.width * 3 / 2 && after.height > before.height * 3 / 2, "twice the points, about twice the cell")
        #expect(renderer.gridSize == (20, 4), "the grid the program has is untouched")
        #expect(renderer.text(ofRow: 0).hasPrefix("hello"), "the cells are the same")
        let image = renderer.renderToImage()
        #expect(RowDrawing.lit(image, x: 0..<after.width * 5, y: 0..<after.height, background: RowDrawing.background), "and they are drawn")
        #expect(renderer.needsDraw, "a change of font is something to draw")
    }

    @Test func aSizeIsKeptWithinWhatSettingsAllow() throws {
        let renderer = try RowDrawing.renderer()
        renderer.setPointSize(2)
        #expect(renderer.configuration.fonts.pointSize == 8)
        renderer.setPointSize(100)
        #expect(renderer.configuration.fonts.pointSize == 36)
    }

    @Test func goingBackToAnEarlierFontFindsItsGlyphsAlreadyRasterised() throws {
        let renderer = try RowDrawing.renderer()
        try renderer.apply(frame: RowDrawing.frame(rows("hello world")))
        renderer.setPointSize(20)
        let first = renderer.counters
        renderer.setPointSize(13)
        let back = renderer.counters - first
        #expect(back.glyphsRasterised == 0 && back.wordsShaped == 0, "13 pt has its atlas and words still")
        #expect(renderer.counters.glyphsRasterised > 0, "the counters add up over every font there has been")
    }

    @Test func aLineHeightChangesTheCellHeightOnly() throws {
        let renderer = try RowDrawing.renderer(TerminalFontSettings(pointSize: 14, lineHeight: 1.0))
        let tight = renderer.cellPixels
        renderer.setFonts(TerminalFontSettings(pointSize: 14, lineHeight: 1.6))
        #expect(renderer.cellPixels.width == tight.width)
        #expect(renderer.cellPixels.height > tight.height)
    }

    @Test func theCursorAndUnderlineComeFromTheFont() throws {
        let small = FontSet(settings: TerminalFontSettings(pointSize: 10), scale: 2).metrics
        let large = FontSet(settings: TerminalFontSettings(pointSize: 36), scale: 2).metrics
        #expect(large.lineThickness > small.lineThickness, "a heavier font draws heavier lines")
        #expect(large.underlineTop > small.underlineTop)
        #expect(large.strikeTop > small.strikeTop)
    }

    @Test func aLigatureIsDrawnWhenOnAndTheSeparateCharactersWhenOff() throws {
        guard let font = RowDrawing.jetBrainsMono else { return }
        func image(ligatures: Bool) throws -> RenderedImage {
            let renderer = try RowDrawing.renderer(TerminalFontSettings(font: font, pointSize: 14, lineHeight: 1.3, ligatures: ligatures))
            try renderer.apply(frame: RowDrawing.frame(rows("a -> b != c")))
            return renderer.renderToImage()
        }
        let on = try image(ligatures: true), off = try image(ligatures: false)
        #expect(on.bgra != off.bgra, "JetBrains Mono joins -> and != through contextual alternates")
        // Nothing but the pairs differs: the first cell ('a') is the same either way.
        let cell = try RowDrawing.renderer(TerminalFontSettings(font: font, pointSize: 14, lineHeight: 1.3)).cellPixels
        for y in 0..<cell.height { for x in 0..<cell.width { #expect(on.pixel(x, y) == off.pixel(x, y)) } }
    }

    @Test func aFallbackFamilyIsAskedForWhatTheFontLacks() throws {
        let renderer = try RowDrawing.renderer(TerminalFontSettings(pointSize: 14, fallbacks: ["Hiragino Sans"]))
        try renderer.apply(frame: RowDrawing.frame(rows("a\u{3042}b")))
        #expect(renderer.counters.glyphsRasterised >= 2)
    }
}
