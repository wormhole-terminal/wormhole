// The placements section of a packed frame: after the rows, when the header's flags carry bit 1; the header's last
// word is the record count. Records are 16 little-endian words: key, z, line, column, cols, rows, source x y w h,
// offset x y, extent kind (0 fill, 1 fit, 2 pixels), extent width, height, one reserved. A frame without the flag
// says the list is as before; a full frame always carries it. The layout version stays 1: a reader that does not
// know the section reads the rows and stops.

import Foundation

public enum PlacementRecords {
    public static let flag: UInt32 = 2
    public static let recordBytes = 64

    /// The list a frame carries, or `nil` when it says "as before".
    public static func read(_ bytes: UnsafeRawBufferPointer, header: FrameHeader) throws(FrameError) -> [ImagePlacement]? {
        guard let count = header.placementCount else { return nil }
        let start = FrameLayout.headerBytes + header.changedRows * (FrameLayout.rowHeaderBytes + header.cols * FrameLayout.cellBytes)
        guard bytes.count >= start + count * recordBytes else { throw .truncated }
        return (0..<count).map { record(bytes, at: start + $0 * recordBytes) }
    }

    private static func record(_ bytes: UnsafeRawBufferPointer, at offset: Int) -> ImagePlacement {
        func word(_ index: Int) -> UInt32 { UInt32(littleEndian: bytes.loadUnaligned(fromByteOffset: offset + index * 4, as: UInt32.self)) }
        func number(_ index: Int) -> Int { Int(word(index)) }
        func signed(_ index: Int) -> Int { Int(Int32(bitPattern: word(index))) }
        let extent: ImageExtent =
            switch word(12) {
            case 1: .fit
            case 2: .pixels(width: number(13), height: number(14))
            default: .fill
            }
        return ImagePlacement(
            key: word(0), z: Int32(bitPattern: word(1)), line: signed(2), column: number(3), cols: number(4), rows: number(5),
            source: ImageRect(x: number(6), y: number(7), width: number(8), height: number(9)),
            offsetX: number(10), offsetY: number(11), extent: extent)
    }

    /// Adds the section to a frame written by `FrameWriter` (the mock's way out, and the tests').
    public static func append(_ placements: [ImagePlacement], to frame: inout Data) {
        var words = Data(capacity: placements.count * recordBytes)
        for p in placements {
            let (kind, width, height): (UInt32, Int, Int) =
                switch p.extent {
                case .fill: (0, 0, 0)
                case .fit: (1, 0, 0)
                case .pixels(let width, let height): (2, width, height)
                }
            let values: [UInt32] = [
                p.key, UInt32(bitPattern: p.z), UInt32(bitPattern: Int32(p.line)), UInt32(p.column), UInt32(p.cols), UInt32(p.rows),
                UInt32(p.source.x), UInt32(p.source.y), UInt32(p.source.width), UInt32(p.source.height),
                UInt32(p.offsetX), UInt32(p.offsetY), kind, UInt32(width), UInt32(height), 0,
            ]
            for value in values { withUnsafeBytes(of: value.littleEndian) { words.append(contentsOf: $0) } }
        }
        frame.append(words)
        patch(&frame, word: 5) { $0 | flag }
        patch(&frame, word: 11) { _ in UInt32(placements.count) }
    }

    private static func patch(_ frame: inout Data, word: Int, _ change: (UInt32) -> UInt32) {
        let at = frame.startIndex + word * 4
        let old = frame[at..<at + 4].withUnsafeBytes { UInt32(littleEndian: $0.loadUnaligned(as: UInt32.self)) }
        let value = change(old).littleEndian
        withUnsafeBytes(of: value) { frame.replaceSubrange(at..<at + 4, with: $0) }
    }
}
