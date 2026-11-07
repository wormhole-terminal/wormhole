import Foundation
import Testing

@testable import TerminalCore

@Suite struct FrameTests {
    @Test func aWrittenFrameReadsBackCellForCell() throws {
        var cells = (0..<12).map { PackedCell(scalar: 0x41 + UInt32($0), fg: PackedColor.rgb(1, 2, 3), bg: PackedColor.indexed(9), flags: .bold) }
        cells[3].flags |= CellFlags.wideChar.rawValue
        let header = FrameHeader(
            cols: 6, rows: 4, changedRows: 2, isFull: false,
            cursor: .init(row: 1, column: 2, shape: .beam, wide: true, blinking: true), displayOffset: 7, historySize: 99)
        var writer = FrameWriter(header: header, rowCapacity: 2)
        cells.withUnsafeBufferPointer { all in
            writer.appendRow(index: 1, size: .single, cells: UnsafeBufferPointer(rebasing: all[0..<6]))
            writer.appendRow(index: 3, size: .doubleWidth, cells: UnsafeBufferPointer(rebasing: all[6..<12]))
        }
        #expect(writer.bytes.count == FrameLayout.headerBytes + 2 * (FrameLayout.rowHeaderBytes + 6 * FrameLayout.cellBytes))

        var seen: [(Int, LineSize, [PackedCell])] = []
        let read = try writer.bytes.withUnsafeBytes { raw in
            let read = try FrameReader.header(raw)
            try FrameReader.forEachRow(raw, header: read) { row, size, cells in seen.append((row, size, Array(cells))) }
            return read
        }
        #expect(read == header)
        #expect(seen.map(\.0) == [1, 3])
        #expect(seen.map(\.1) == [.single, .doubleWidth])
        #expect(seen[0].2 == Array(cells[0..<6]))
        #expect(seen[1].2 == Array(cells[6..<12]))
    }

    @Test func aPackedCellIsSixteenBytesInTheEnginesOrder() {
        #expect(MemoryLayout<PackedCell>.size == 16)
        #expect(MemoryLayout<PackedCell>.stride == 16)
        #expect(MemoryLayout<PackedCell>.offset(of: \.scalar) == 0)
        #expect(MemoryLayout<PackedCell>.offset(of: \.flags) == 12)
    }

    @Test func theRustSideTestsBytesParse() throws {
        // The layout test in `wormhole-ffi`'s frame.rs: header words by name, then a row.
        var data = Data()
        for word: UInt32 in [FrameLayout.magic, 1, 10, 4, 1, 0, UInt32.max, 0, 0, 0, 0, 0] {
            withUnsafeBytes(of: word.littleEndian) { data.append(contentsOf: $0) }
        }
        for word: UInt32 in [1, 0] { withUnsafeBytes(of: word.littleEndian) { data.append(contentsOf: $0) } }
        for _ in 0..<10 {
            for word: UInt32 in [0x78, 257, 1, 0] { withUnsafeBytes(of: word.littleEndian) { data.append(contentsOf: $0) } }
        }
        let header = try data.withUnsafeBytes { try FrameReader.header($0) }
        #expect(header.cols == 10 && header.rows == 4 && header.changedRows == 1 && header.cursor == nil)
    }

    @Test func garbageIsRefusedNotTrusted() {
        #expect(throws: FrameError.truncated) { try Data(count: 8).withUnsafeBytes { try FrameReader.header($0) } }
        #expect(throws: FrameError.badMagic) { try Data(count: 48).withUnsafeBytes { try FrameReader.header($0) } }
        var data = Data(count: 0)
        for word: UInt32 in [FrameLayout.magic, 9, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0] {
            withUnsafeBytes(of: word.littleEndian) { data.append(contentsOf: $0) }
        }
        #expect(throws: FrameError.unsupportedVersion(9)) { try data.withUnsafeBytes { try FrameReader.header($0) } }
    }
}

@Suite struct MockTests {
    private func decode(_ data: Data) throws -> (FrameHeader, [Int: [PackedCell]]) {
        var rows: [Int: [PackedCell]] = [:]
        let header = try data.withUnsafeBytes { raw -> FrameHeader in
            let header = try FrameReader.header(raw)
            try FrameReader.forEachRow(raw, header: header) { row, _, cells in rows[row] = Array(cells) }
            return header
        }
        return (header, rows)
    }

    @Test func theFirstFrameIsEveryRowAndTheNextOnlyWhatChanged() throws {
        let source = MockFrameSource(.init(scenario: .mixed, manual: true), size: GridSize(cols: 40, rows: 20))
        try source.start(size: GridSize(cols: 40, rows: 20), wake: {})
        let (first, rows) = try decode(source.frame())
        #expect(first.isFull && first.changedRows == 20 && rows.count == 20)
        let (idle, _) = try decode(source.frame())
        #expect(idle.changedRows == 0 && !idle.isFull)
        source.step()
        let (next, _) = try decode(source.frame())
        // Scrolling text below the gallery damages its region, and the spinner the first row.
        #expect(next.changedRows > 0 && next.changedRows < 20)
    }

    @Test func aSameSeedGivesTheSameBytes() throws {
        func run() throws -> Data {
            let source = MockFrameSource(.init(scenario: .flood, seed: 5, manual: true), size: GridSize(cols: 30, rows: 10))
            try source.start(size: GridSize(cols: 30, rows: 10), wake: {})
            for _ in 0..<5 { source.step() }
            return source.frame()
        }
        #expect(try run() == run())
    }

    @Test func aFloodOfOutputWakesOnceUntilDrained() throws {
        let woken = Mutex2(0)
        let source = MockFrameSource(.init(scenario: .text, manual: true), size: GridSize(cols: 20, rows: 5))
        try source.start(size: GridSize(cols: 20, rows: 5), wake: { woken.add() })
        for _ in 0..<100 { source.step() }
        #expect(woken.value == 1)
        let events = source.drainEvents()
        #expect(events.filter { $0 == .output }.count == 1, "one Output for the flood")
        source.step()
        #expect(woken.value == 2, "the next event after a drain wakes again")
    }

    @Test func wideCharactersTakeTwoCellsAndASpacer() throws {
        var screen = MockScreen(cols: 10, rows: 2)
        screen.write("a日b")
        #expect(screen[0, 0].scalar == 0x61)
        #expect(screen[0, 1].cellFlags.contains(.wideChar) && screen[0, 1].scalar == 0x65E5)
        #expect(screen[0, 2].cellFlags.contains(.wideCharSpacer))
        #expect(screen[0, 3].scalar == 0x62)
    }

    @Test func scrollingTheViewBackSendsAWholeFrameWithTheOffset() throws {
        let source = MockFrameSource(.init(scenario: .text, manual: true), size: GridSize(cols: 20, rows: 5))
        try source.start(size: GridSize(cols: 20, rows: 5), wake: {})
        for _ in 0..<30 { source.step() }
        _ = source.frame()
        source.scroll(lines: 4)
        let (header, rows) = try decode(source.frame())
        #expect(header.displayOffset == 4 && header.isFull && rows.count == 5 && header.cursor == nil)
    }
}

/// A counter the wake closure (`@Sendable`) can bump from any thread.
final class Mutex2: @unchecked Sendable {
    private let lock = NSLock()
    private var count: Int
    init(_ count: Int) { self.count = count }
    func add() { lock.lock(); count += 1; lock.unlock() }
    var value: Int { lock.lock(); defer { lock.unlock() }; return count }
}
