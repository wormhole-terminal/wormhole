import Foundation
import Testing
import TerminalCore

@testable import TerminalMetal

/// Find's matches painted from the palette, and blinking text on the shared clock.
@MainActor
@Suite struct FindAndBlinkTests {
    private let palette = TerminalPalette.spike

    private func pixel(_ renderer: MetalRenderer, column: Int, row: Int = 0) -> (r: UInt8, g: UInt8, b: UInt8) {
        let cell = renderer.cellPixels
        // The cell's top-left corner is background or the match's fill, never a glyph's stroke.
        return renderer.renderToImage().pixel(renderer.left(column) + 1, row * cell.height + 1)
    }

    private func word(_ renderer: MetalRenderer) throws {
        var rows = RowDrawing.blank()
        for (column, scalar) in "match match".unicodeScalars.enumerated() { rows[0][column] = PackedCell(scalar: scalar.value) }
        try renderer.apply(frame: RowDrawing.frame(rows))
    }

    @Test func theCurrentMatchTakesTheFindFillAndTheOthersTintTheirBackground() throws {
        let renderer = try RowDrawing.renderer()
        try word(renderer)
        renderer.setFound([FindSpan(row: 0, first: 0, last: 4, isCurrent: true), FindSpan(row: 0, first: 6, last: 10)])
        let current = palette.findCurrent
        #expect(pixel(renderer, column: 0) == (current.x, current.y, current.z))
        let tint = palette.resolve(palette.tinted(PackedColor.background))
        #expect(pixel(renderer, column: 6) == (tint.x, tint.y, tint.z))
        let background = palette.background
        #expect(pixel(renderer, column: 5) == (background.x, background.y, background.z), "between the matches nothing changes")
        #expect(tint != background, "the tint shows")
    }

    @Test func aTintBlendsOverTheCellsOwnBackgroundAndNotOverASelection() throws {
        let renderer = try RowDrawing.renderer()
        var rows = RowDrawing.blank()
        rows[0][0] = PackedCell(bg: PackedColor.rgb(200, 0, 0))
        rows[0][1] = PackedCell()
        try renderer.apply(frame: RowDrawing.frame(rows))
        renderer.setSelection([RowSpan(row: 0, first: 1, last: 1)])
        renderer.setFound([FindSpan(row: 0, first: 0, last: 1)])
        let red = palette.resolve(palette.tinted(PackedColor.rgb(200, 0, 0)))
        #expect(pixel(renderer, column: 0) == (red.x, red.y, red.z))
        #expect(red != SIMD3<UInt8>(200, 0, 0), "the match's colour is mixed in")
        let selection = palette.selectionBackground
        #expect(pixel(renderer, column: 1) == (selection.x, selection.y, selection.z), "the selection keeps its colour")
    }

    @Test func aNewPaletteRepaintsTheMatches() throws {
        let renderer = try RowDrawing.renderer()
        try word(renderer)
        renderer.setFound([FindSpan(row: 0, first: 0, last: 4, isCurrent: true)])
        let colours = TerminalPalette.FindColors(match: SIMD3(0, 0, 255), matchAlpha: 0.5, current: SIMD3(1, 2, 3), currentText: SIMD3(9, 9, 9))
        renderer.setPalette(TerminalPalette(ansi: Array(repeating: SIMD3(1, 1, 1), count: 16), foreground: SIMD3(250, 250, 250), background: SIMD3(0, 0, 0), cursor: SIMD3(1, 1, 1), find: colours))
        #expect(pixel(renderer, column: 0) == (1, 2, 3))
    }

    @Test func blinkingTextFadesInTheOffHalfAndTheRowsAreNotRebuilt() throws {
        let renderer = try RowDrawing.renderer()
        var rows = RowDrawing.blank()
        rows[0][0] = PackedCell(scalar: 0x2588, fg: PackedColor.rgb(200, 200, 200), flags: .blink)
        try renderer.apply(frame: RowDrawing.frame(rows))
        #expect(renderer.hasBlinkingText)
        let cell = renderer.cellPixels
        func level() -> UInt8 { renderer.renderToImage().pixel(renderer.left(1) / 2, cell.height / 2).r }
        let on = level()
        let built = renderer.counters
        renderer.setBlinkFaded(true)
        #expect(renderer.needsDraw)
        let off = level()
        #expect(on == 200)
        #expect(off < on / 2 + 40 && off > 0, "faded, not gone: \(off)")
        #expect((renderer.counters - built).rowsRebuilt == 0, "a flip rebuilds nothing")
    }

    @Test func steadyTextDoesNotAskForTheClock() throws {
        let renderer = try RowDrawing.renderer()
        try renderer.apply(frame: RowDrawing.frame(RowDrawing.blank()))
        #expect(!renderer.hasBlinkingText)
    }

    @Test func theBlinkClockHasEqualHalvesAndFlipsAtTheirEnds() {
        let start = TextBlinkClock.start
        #expect(!TextBlinkClock.isFaded(at: start + .milliseconds(100)), "on for the first half")
        #expect(TextBlinkClock.isFaded(at: start + .milliseconds(600)), "off for the second")
        #expect(!TextBlinkClock.isFaded(at: start + .milliseconds(1100)))
        #expect(TextBlinkClock.untilFlip(from: start + .milliseconds(100)) == .milliseconds(400))
        #expect(TextBlinkClock.untilFlip(from: start + .milliseconds(600)) == .milliseconds(400))
    }
}
