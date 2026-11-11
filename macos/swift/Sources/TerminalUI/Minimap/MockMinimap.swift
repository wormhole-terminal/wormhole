// A minimap over a synthetic history, for the gallery and tests: lines of words in two colours with a prompt
// and a coloured block now and then; scrolling is remembered, not acted on.

import Foundation
import TerminalCore

@MainActor
public final class MockMinimap: MinimapSource {
    public let palette = TerminalPalette.spike
    public private(set) var displayOffset: Int
    public let history: Int
    public let screen = 30
    public let columns = 100

    public init(history: Int = 400, displayOffset: Int = 120) { (self.history, self.displayOffset) = (history, displayOffset) }

    public func snapshot(fits: Int) -> MinimapSnapshot {
        let total = history + screen
        let geometry = MinimapGeometry(total: total, screen: screen, fits: fits, viewTop: history - min(displayOffset, history))
        let count = min(fits, total - geometry.first)
        return MinimapSnapshot(geometry: geometry, columns: columns, lines: (0..<count).map { runs(forLine: geometry.first + $0) })
    }

    public func scroll(toDisplayOffset offset: Int) { displayOffset = offset }

    private func runs(forLine line: Int) -> [MinimapRun] {
        let ansi = [PackedColor.named(2), PackedColor.named(4), PackedColor.named(3), PackedColor.foreground]
        let isPrompt = line % 9 == 0
        var runs: [MinimapRun] = []
        var column = isPrompt ? 2 : 4 * (line % 3)
        var word = line
        while column < columns - 6 {
            let length = 3 + (word * 7) % 9
            let fg = isPrompt && runs.isEmpty ? ansi[0] : ansi[(word / 3) % ansi.count]
            runs.append(MinimapRun(start: column, end: min(column + length, columns), isText: true, cell: PackedCell(scalar: 0x61, fg: fg)))
            column += length + 1 + word % 2
            word += 5
        }
        if line % 23 == 0 {
            runs.append(MinimapRun(start: 0, end: columns, isText: false, cell: PackedCell(bg: PackedColor.named(1))))
        }
        return runs
    }
}
