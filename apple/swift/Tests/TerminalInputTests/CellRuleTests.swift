#if WITH_UNIFFI
import Foundation
import Testing
import TerminalCore
import UniFFISource

@testable import TerminalMetal

/// The cell is the core's (`view::layout::cell`, the GPUI app's rule): the renderer's grid for the bundled font is GPUI's, and
/// the arithmetic a build without Rust uses cannot drift from it.
@MainActor
@Suite(.serialized) struct CellRuleTests {
    @Test func jetBrainsMonoAt14PointsIsGPUIsCell() throws {
        installCellRules()
        var directory = URL(fileURLWithPath: #filePath).deletingLastPathComponent()
        while !FileManager.default.fileExists(atPath: directory.appendingPathComponent("crates/wormhole-core/assets/fonts/JetBrainsMono").path) {
            directory.deleteLastPathComponent()
        }
        let font = FontSpec.jetBrainsMono(directory: directory.appendingPathComponent("crates/wormhole-core/assets/fonts/JetBrainsMono"))
        let fonts = FontSet(settings: TerminalFontSettings(font: font, pointSize: 14, lineHeight: 1.3), scale: 2)
        // 600/1000 em: 8.4 pt, 16.8 px; 14 x 1.3 rounded: 18 pt; the baseline centred: 14.04 pt, pixel 28.
        #expect(abs(fonts.cell.width - 8.4) < 1e-4 && fonts.cell.height == 18)
        #expect(abs(fonts.metrics.width - 16.8) < 1e-3 && fonts.metrics.height == 36 && fonts.metrics.baseline == 28)
        // A 1320 x 820 window's pane in both apps: the same whole cells.
        #expect(CellRule.current.fit(fonts.cell, width: 952, height: 774, slack: 0) == (113, 43))
        #expect((0...6).map(fonts.metrics.edge) == [0, 17, 34, 50, 67, 84, 101])
    }

    @Test func theArithmeticWithoutRustIsTheCores() {
        installCellRules()
        let plain = CellRule.Plain(), core = CellRule.current
        for size in [9.0, 12.5, 14, 17.25, 36] {
            for (advance, ascent, descent) in [(0.6, 1.02, 0.3), (0.602, 0.928, 0.236), (0.5, 0.8, 0.2)] {
                for line in [1.0, 1.2, 1.3, 1.6] {
                    let a = plain.cell(fontSize: size, lineHeight: line, advance: advance * size, ascent: ascent * size, descent: descent * size)
                    let b = core.cell(fontSize: size, lineHeight: line, advance: advance * size, ascent: ascent * size, descent: descent * size)
                    #expect(a == b, "\(size) pt, line \(line)")
                    #expect(plain.fit(a, width: 951.7, height: 773.9, slack: 0.5) == core.fit(b, width: 951.7, height: 773.9, slack: 0.5))
                }
            }
        }
    }
}
#endif
