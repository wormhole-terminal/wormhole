import Foundation
import Testing

@testable import TerminalCore
@testable import TerminalMetal

/// The two sections Rust writes after the cell extras (`probes/marks.rs`), what the scene makes of them, and the rules for
/// painting a Probe's marks under the selection.
@Suite struct ProbeSectionsTests {
    private func words(_ values: [UInt32]) -> Data {
        var data = Data()
        for word in values { withUnsafeBytes(of: word.littleEndian) { data.append(contentsOf: $0) } }
        return data
    }

    /// A frame with no rows and `section` after them, flagged as `flags` say.
    private func frame(flags: UInt32, _ section: Data) -> Data {
        var data = FrameWriter(header: FrameHeader(cols: 4, rows: 2, changedRows: 0, isFull: false, cursor: nil), rowCapacity: 0).bytes
        data.append(section)
        data.withUnsafeMutableBytes { $0.storeBytes(of: flags.littleEndian, toByteOffset: 20, as: UInt32.self) }
        return data
    }

    private func read(_ data: Data) throws -> FrameSections {
        try data.withUnsafeBytes { try FrameSections.read($0, header: try FrameReader.header($0)) }
    }

    private func text(_ string: String) -> Data {
        var data = words([UInt32(string.utf8.count)])
        data.append(contentsOf: string.utf8)
        data.append(Data(count: (4 - string.utf8.count % 4) % 4))
        return data
    }

    @Test func decorationsAndMarkersComeBackByRowWithTheirWordsAndTones() throws {
        var section = words([1, /* row */ 1, /* block */ 1, /* spans */ 2])
        section.append(words([0, 1, 2, 0]))  // columns 0...1, danger, the text stays
        section.append(words([2, 3, 0, 2, 0x62, 0x2026]))  // columns 2...3, no tone, masked "b…"
        section.append(words([1, /* row */ 0, 7, 0, /* hidden */ 124, /* open | failed */ 2]))
        section.append(text("124 lines hidden · build output"))
        section.append(text("124 lines hidden by Demo, build output"))
        section.append(words([UInt32.max]))  // no tooltip
        let found = try read(frame(flags: 8 | 16, section))
        #expect(found.decor[1] == RowDecor(block: .link, spans: [RuleSpan(first: 0, last: 1, tone: .danger), RuleSpan(first: 2, last: 3, mask: [0x62, 0x2026])]))
        let marker = try #require(found.markers[0])
        #expect(marker.id == 7 && marker.hidden == 124 && !marker.isOpen && marker.failed)
        #expect(marker.label == "124 lines hidden · build output" && marker.spoken.hasPrefix("124 lines hidden by Demo") && marker.tooltip == nil)
    }

    @Test func aFrameWithoutTheFlagsHasNeitherAndATruncatedSectionIsRefused() throws {
        #expect(try read(frame(flags: 0, Data())).decor.isEmpty)
        #expect(throws: FrameError.truncated) { try read(frame(flags: 8, words([1, 0, 0, 1, 0, 1]))) }
    }

    @Test func aMaskReplacesTheTextAndATonePaintsItUnlessTheSelectionStandsOver() {
        var cells = "key sk-abcdef".unicodeScalars.map { PackedCell(scalar: $0.value) }
        var marks = RowMarks()
        marks.decor = RowDecor(block: nil, spans: [RuleSpan(first: 4, last: 12, tone: .danger, mask: Array(repeating: 0x2588, count: 9))])
        marks.selected = 10...12
        marks.apply(to: &cells, palette: .spike)
        #expect(cells[4].scalar == 0x2588 && cells[4].fg == PackedColor.tone(.danger))
        #expect(cells[0].scalar == UInt32(("k" as Unicode.Scalar).value), "text outside the span stays")
        #expect(cells[11].fg != PackedColor.tone(.danger), "the selection's colours stand over a tone")
        #expect(cells[11].bg == PackedColor.selectionBackground)
    }

    @Test func aBlocksTintIsFaintAndKeepsTheTextsOwnColour() {
        var cells = [PackedCell(scalar: 0x78)]
        var marks = RowMarks()
        marks.decor = RowDecor(block: .danger)
        marks.apply(to: &cells, palette: .spike)
        let plain = TerminalPalette.spike.resolve(PackedColor.background), tinted = TerminalPalette.spike.resolve(cells[0].bg)
        #expect(tinted != plain && cells[0].fg == PackedColor.foreground)
        let drift = SIMD3<Int>(truncatingIfNeeded: tinted) &- SIMD3<Int>(truncatingIfNeeded: plain)
        #expect(max(abs(drift.x), abs(drift.y), abs(drift.z)) < 60, "16 % of a tone is a tint, not a fill")
    }

    @Test func aMaskedCellsMarksAreNotShapedWithIt() {
        let decor = RowDecor(spans: [RuleSpan(first: 1, last: 2, mask: [0x2588, 0x2588])])
        let kept = decor.withoutMasked([0: CellExtra(marks: [0x301]), 1: CellExtra(marks: [0x301])])
        #expect(kept?[0] != nil && kept?[1] == nil)
    }
}
