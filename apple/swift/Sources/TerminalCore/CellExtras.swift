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
        var words = FrameWords(bytes, header: header)
        return try read(&words, header: header)
    }

    static func read(_ words: inout FrameWords, header: FrameHeader) throws(FrameError) -> [Int: [Int: CellExtra]] {
        guard header.hasExtras else { return [:] }
        var found: [Int: [Int: CellExtra]] = [:]
        for _ in 0..<(try words.next()) {
            let (row, column, underline, count) = (try words.next(), try words.next(), try words.next(), try words.next())
            var marks: [UInt32] = []
            for _ in 0..<count { marks.append(UInt32(try words.next())) }
            found[row, default: [:]][column] = CellExtra(underline: underline == Int(noColour) ? nil : UInt32(underline), marks: marks)
        }
        return found
    }
}
