// The packed frame: what `Terminal::frame()` returns on the Rust side (`wormhole-ffi`'s `frame.rs`) and
// what the mock writes, read here without copying into Swift types.
//
//   header   48 bytes of little-endian u32: magic, version, cols, rows, changed rows, flags,
//            cursor row (`noCursor` if hidden), cursor column, cursor style, display offset,
//            history size, 2 reserved
//   per changed row: row index (u32), line size (u32), then `cols` cells of 16 bytes
//   cell     char (u32), fg, bg, flags (the engine's bits, `CellFlags`)
//   colour   tag << 24 | value: tag 0 a named slot (0..15 ANSI, 256 foreground, 257 background,
//            258 cursor), tag 1 an indexed colour (0..255), tag 2 truecolor (r << 16 | g << 8 | b)
//
// Colours stay unresolved so a theme change rebuilds no row: the renderer resolves them against its palette.

import Foundation

public enum FrameLayout {
    /// `u32::from_le_bytes(*b"WHF1")`.
    public static let magic: UInt32 = 0x3146_4857
    /// The layout version this reader understands; the library's `frame_format_version()` must equal it.
    public static let version: UInt32 = 1
    public static let headerBytes = 48
    public static let rowHeaderBytes = 8
    public static let cellBytes = 16
    public static let noCursor = UInt32.max
}

/// The engine's cell flags (`wormhole-vt` `types/cell.rs`), same bit positions.
public struct CellFlags: OptionSet, Sendable {
    public let rawValue: UInt32
    public init(rawValue: UInt32) { self.rawValue = rawValue }

    public static let bold = CellFlags(rawValue: 1)
    public static let dim = CellFlags(rawValue: 1 << 1)
    public static let italic = CellFlags(rawValue: 1 << 2)
    public static let underline = CellFlags(rawValue: 1 << 3)
    public static let doubleUnderline = CellFlags(rawValue: 1 << 4)
    public static let undercurl = CellFlags(rawValue: 1 << 5)
    public static let dottedUnderline = CellFlags(rawValue: 1 << 6)
    public static let dashedUnderline = CellFlags(rawValue: 1 << 7)
    public static let blink = CellFlags(rawValue: 1 << 8)
    public static let inverse = CellFlags(rawValue: 1 << 9)
    public static let hidden = CellFlags(rawValue: 1 << 10)
    public static let strikeout = CellFlags(rawValue: 1 << 11)
    public static let overline = CellFlags(rawValue: 1 << 12)
    public static let wideChar = CellFlags(rawValue: 1 << 13)
    public static let wideCharSpacer = CellFlags(rawValue: 1 << 14)
    public static let leadingWideCharSpacer = CellFlags(rawValue: 1 << 15)
    public static let wrapLine = CellFlags(rawValue: 1 << 16)

    public static let allUnderlines: CellFlags = [
        .underline, .doubleUnderline, .undercurl, .dottedUnderline, .dashedUnderline,
    ]
}

/// One cell as it sits in the frame: sixteen bytes, `Copy`, no padding.
public struct PackedCell: Sendable, Equatable {
    public var scalar: UInt32
    public var fg: UInt32
    public var bg: UInt32
    public var flags: UInt32

    public init(scalar: UInt32 = 0x20, fg: UInt32 = PackedColor.foreground, bg: UInt32 = PackedColor.background, flags: CellFlags = []) {
        self.scalar = scalar
        self.fg = fg
        self.bg = bg
        self.flags = flags.rawValue
    }

    @inline(__always) public var cellFlags: CellFlags { CellFlags(rawValue: flags) }
}

/// Helpers for the tag-and-value colour words.
public enum PackedColor {
    public static let foreground: UInt32 = 256
    public static let background: UInt32 = 257
    public static let cursor: UInt32 = 258

    @inline(__always) public static func named(_ slot: Int) -> UInt32 { UInt32(slot) }
    @inline(__always) public static func indexed(_ index: Int) -> UInt32 { 1 << 24 | UInt32(index) }
    @inline(__always) public static func rgb(_ r: UInt8, _ g: UInt8, _ b: UInt8) -> UInt32 {
        2 << 24 | UInt32(r) << 16 | UInt32(g) << 8 | UInt32(b)
    }
}

/// How a row is drawn (the engine's `LineSize`).
public enum LineSize: UInt32, Sendable {
    case single = 0
    case doubleWidth = 1
    case doubleHeightTop = 2
    case doubleHeightBottom = 3
}

public enum CursorShape: UInt32, Sendable {
    case block = 0
    case underline = 1
    case beam = 2
}

public struct FrameHeader: Sendable, Equatable {
    public var cols: Int
    public var rows: Int
    public var changedRows: Int
    /// Every row is in the frame: the first one, a resize, a scroll.
    public var isFull: Bool
    /// `nil` when the cursor is hidden or scrolled out of view.
    public var cursor: Cursor?
    public var displayOffset: Int
    public var historySize: Int

    public struct Cursor: Sendable, Equatable {
        public var row: Int
        public var column: Int
        public var shape: CursorShape
        public var wide: Bool
        public var blinking: Bool

        public init(row: Int, column: Int, shape: CursorShape = .block, wide: Bool = false, blinking: Bool = false) {
            self.row = row
            self.column = column
            self.shape = shape
            self.wide = wide
            self.blinking = blinking
        }
    }

    public init(
        cols: Int, rows: Int, changedRows: Int, isFull: Bool, cursor: Cursor?, displayOffset: Int = 0,
        historySize: Int = 0
    ) {
        self.cols = cols
        self.rows = rows
        self.changedRows = changedRows
        self.isFull = isFull
        self.cursor = cursor
        self.displayOffset = displayOffset
        self.historySize = historySize
    }
}

public enum FrameError: Error, Equatable {
    case truncated
    case badMagic
    case unsupportedVersion(UInt32)
    case misaligned
    case rowOutOfRange(Int)
}

public enum FrameReader {
    /// Check and decode the header. An empty buffer (a closed terminal) is a frame with no rows.
    public static func header(_ bytes: UnsafeRawBufferPointer) throws(FrameError) -> FrameHeader {
        guard bytes.count >= FrameLayout.headerBytes else { throw .truncated }
        guard Int(bitPattern: bytes.baseAddress) % MemoryLayout<UInt32>.alignment == 0 else { throw .misaligned }
        func word(_ index: Int) -> UInt32 { UInt32(littleEndian: bytes.load(fromByteOffset: index * 4, as: UInt32.self)) }
        guard word(0) == FrameLayout.magic else { throw .badMagic }
        guard word(1) == FrameLayout.version else { throw .unsupportedVersion(word(1)) }
        let cursorRow = word(6)
        let style = word(8)
        let cursor: FrameHeader.Cursor? =
            cursorRow == FrameLayout.noCursor
            ? nil
            : FrameHeader.Cursor(
                row: Int(cursorRow), column: Int(word(7)), shape: CursorShape(rawValue: style & 0xFF) ?? .block,
                wide: style & 1 << 8 != 0, blinking: style & 1 << 9 != 0)
        return FrameHeader(
            cols: Int(word(2)), rows: Int(word(3)), changedRows: Int(word(4)), isFull: word(5) & 1 != 0,
            cursor: cursor, displayOffset: Int(word(9)), historySize: Int(word(10)))
    }

    /// Visit each changed row in order. `body` gets the row index, its line size and its cells, which
    /// point into `bytes` (copy what must outlive the call).
    public static func forEachRow(
        _ bytes: UnsafeRawBufferPointer, header: FrameHeader,
        _ body: (_ row: Int, _ size: LineSize, _ cells: UnsafeBufferPointer<PackedCell>) -> Void
    ) throws(FrameError) {
        let perRow = FrameLayout.rowHeaderBytes + header.cols * FrameLayout.cellBytes
        guard bytes.count >= FrameLayout.headerBytes + header.changedRows * perRow else { throw .truncated }
        var offset = FrameLayout.headerBytes
        for _ in 0..<header.changedRows {
            let row = Int(UInt32(littleEndian: bytes.load(fromByteOffset: offset, as: UInt32.self)))
            let sizeWord = UInt32(littleEndian: bytes.load(fromByteOffset: offset + 4, as: UInt32.self))
            guard row < header.rows else { throw .rowOutOfRange(row) }
            let start = bytes.baseAddress!.advanced(by: offset + FrameLayout.rowHeaderBytes)
            let cells = UnsafeBufferPointer(
                start: start.assumingMemoryBound(to: PackedCell.self), count: header.cols)
            body(row, LineSize(rawValue: sizeWord) ?? .single, cells)
            offset += perRow
        }
    }
}

/// Writes frames in the layout above: the mock's way out, and the tests'.
public struct FrameWriter {
    public private(set) var bytes: Data

    public init(header: FrameHeader, rowCapacity: Int) {
        bytes = Data(capacity: FrameLayout.headerBytes + rowCapacity * (FrameLayout.rowHeaderBytes + header.cols * FrameLayout.cellBytes))
        let cursor = header.cursor
        let style: UInt32 =
            (cursor?.shape.rawValue ?? 0) | (cursor?.wide == true ? 1 << 8 : 0) | (cursor?.blinking == true ? 1 << 9 : 0)
        for word: UInt32 in [
            FrameLayout.magic, FrameLayout.version, UInt32(header.cols), UInt32(header.rows),
            UInt32(header.changedRows), header.isFull ? 1 : 0,
            cursor.map { UInt32($0.row) } ?? FrameLayout.noCursor, UInt32(cursor?.column ?? 0), style,
            UInt32(header.displayOffset), UInt32(header.historySize), 0,
        ] {
            append(word)
        }
    }

    public mutating func appendRow(index: Int, size: LineSize, cells: UnsafeBufferPointer<PackedCell>) {
        append(UInt32(index))
        append(size.rawValue)
        bytes.append(UnsafeBufferPointer(start: UnsafeRawPointer(cells.baseAddress!).assumingMemoryBound(to: UInt8.self), count: cells.count * FrameLayout.cellBytes))
    }

    private mutating func append(_ word: UInt32) {
        withUnsafeBytes(of: word.littleEndian) { bytes.append(contentsOf: $0) }
    }
}
