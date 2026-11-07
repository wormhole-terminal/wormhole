// The cell extras section of a packed frame: after the rows and the placements, when the header's flags carry bit 2
// (value 4): a count, then per record row, column, underline colour (`noColour` when the text's own), the number of
// characters n and n scalar values. They are what a cell carries beyond its character: the zero-width characters that
// join into one picture (an accent, a variation selector, a family emoji's joiners, a flag's second letter, a keycap)
// and an underline colour of its own (SGR 58). A changed row without a record has none.

import Foundation

public struct CellExtra: Sendable, Equatable {
    public var underline: UInt32?
    public var marks: [UInt32]

    public init(underline: UInt32? = nil, marks: [UInt32] = []) { (self.underline, self.marks) = (underline, marks) }
}

public enum CellExtras {
    public static let flag: UInt32 = 4
    static let noColour = UInt32.max

    /// The records a frame carries, by row and column; empty without the section.
    public static func read(_ bytes: UnsafeRawBufferPointer, header: FrameHeader) throws(FrameError) -> [Int: [Int: CellExtra]] {
        guard header.hasExtras else { return [:] }
        let rows = header.changedRows * (FrameLayout.rowHeaderBytes + header.cols * FrameLayout.cellBytes)
        var offset = FrameLayout.headerBytes + rows + (header.placementCount ?? 0) * PlacementRecords.recordBytes
        func next() throws(FrameError) -> Int {
            guard offset + 4 <= bytes.count else { throw .truncated }
            defer { offset += 4 }
            return Int(UInt32(littleEndian: bytes.loadUnaligned(fromByteOffset: offset, as: UInt32.self)))
        }
        var found: [Int: [Int: CellExtra]] = [:]
        for _ in 0..<(try next()) {
            let (row, column, underline, count) = (try next(), try next(), try next(), try next())
            var marks: [UInt32] = []
            for _ in 0..<count { marks.append(UInt32(try next())) }
            found[row, default: [:]][column] = CellExtra(underline: underline == Int(noColour) ? nil : UInt32(underline), marks: marks)
        }
        return found
    }
}
