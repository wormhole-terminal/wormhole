import Foundation
import Testing

@testable import TerminalCore

/// The cell extras section the Rust side writes (`frame_extras.rs`): read after the rows and the placements.
@Suite struct CellExtrasTests {
    private func frame(_ words: [UInt32], placements: Int = 0) -> Data {
        var data = FrameWriter(header: FrameHeader(cols: 4, rows: 2, changedRows: 0, isFull: false, cursor: nil), rowCapacity: 0).bytes
        var section = Data()
        for word in words { withUnsafeBytes(of: word.littleEndian) { section.append(contentsOf: $0) } }
        // The placements that come first, as zeroed records; the flags say both sections are there.
        data.append(Data(count: placements * PlacementRecords.recordBytes))
        data.append(section)
        data.withUnsafeMutableBytes { bytes in
            bytes.storeBytes(of: (PlacementRecords.flag * UInt32(placements.signum()) | CellExtras.flag).littleEndian, toByteOffset: 20, as: UInt32.self)
            bytes.storeBytes(of: UInt32(placements).littleEndian, toByteOffset: 44, as: UInt32.self)
        }
        return data
    }

    private func read(_ data: Data) throws -> [Int: [Int: CellExtra]] {
        try data.withUnsafeBytes { try CellExtras.read($0, header: try FrameReader.header($0)) }
    }

    @Test func recordsComeBackByRowAndColumnWithTheirMarksAndColours() throws {
        let none = UInt32.max
        // An accent on (0, 1); a man, joiner, woman on (1, 0); an underline colour of its own on (1, 3).
        let found = try read(frame([3, 0, 1, none, 1, 0x301, 1, 0, none, 2, 0x200D, 0x1F469, 1, 3, 2 << 24 | 0xFF0000, 0]))
        #expect(found[0]?[1] == CellExtra(marks: [0x301]))
        #expect(found[1]?[0] == CellExtra(marks: [0x200D, 0x1F469]))
        #expect(found[1]?[3] == CellExtra(underline: 2 << 24 | 0xFF0000))
    }

    @Test func theSectionFollowsThePlacementsAndATruncatedOneIsRefused() throws {
        #expect(try read(frame([1, 0, 0, UInt32.max, 1, 0x301], placements: 2))[0]?[0]?.marks == [0x301])
        #expect(throws: FrameError.truncated) { try read(frame([1, 0, 0, UInt32.max, 3, 0x301])) }
    }

    @Test func aFrameWithoutTheFlagHasNone() throws {
        let data = FrameWriter(header: FrameHeader(cols: 4, rows: 2, changedRows: 0, isFull: false, cursor: nil), rowCapacity: 0).bytes
        #expect(try read(data).isEmpty)
    }
}
