// A packed frame's sections after the rows are runs of little-endian words (and, for fold markers, padded text):
// this reads them in order, so each section knows only its own record layout and the next one starts where the last ended.

import Foundation

struct FrameWords {
    let bytes: UnsafeRawBufferPointer
    var offset: Int

    /// The words that follow the rows and the placements of a frame with `header`.
    init(_ bytes: UnsafeRawBufferPointer, header: FrameHeader) {
        self.bytes = bytes
        offset =
            FrameLayout.headerBytes + header.changedRows * (FrameLayout.rowHeaderBytes + header.cols * FrameLayout.cellBytes)
            + (header.placementCount ?? 0) * PlacementRecords.recordBytes
    }

    mutating func next() throws(FrameError) -> Int {
        guard offset + 4 <= bytes.count else { throw .truncated }
        defer { offset += 4 }
        return Int(UInt32(littleEndian: bytes.loadUnaligned(fromByteOffset: offset, as: UInt32.self)))
    }

    /// A string: its byte length (`UInt32.max`: none) and its UTF-8, padded to a whole word.
    mutating func text() throws(FrameError) -> String? {
        let length = try next()
        if length == Int(UInt32.max) { return nil }
        guard offset + length <= bytes.count else { throw .truncated }
        defer { offset += (length + 3) / 4 * 4 }
        return String(decoding: UnsafeRawBufferPointer(rebasing: bytes[offset..<offset + length]), as: UTF8.self)
    }
}
