// A screen of packed cells with per-row damage, scrollback and a cursor: the mock's stand-in for the
// engine's grid. It writes frames in the layout the Rust side writes, so the renderer cannot tell them apart.

import Foundation

/// Character widths for the mock and for composed (IME) text. A table of the ranges that matter for the
/// spike; the real engine's width tables are generated from Unicode (`wormhole-vt` `unicode/`).
public enum CellWidth {
    public static func of(_ scalar: UInt32) -> Int {
        switch scalar {
        case 0x1100...0x115F, 0x2E80...0x303E, 0x3041...0x33FF, 0x3400...0x4DBF, 0x4E00...0x9FFF,
            0xA000...0xA4CF, 0xAC00...0xD7A3, 0xF900...0xFAFF, 0xFE30...0xFE6F, 0xFF01...0xFF60,
            0xFFE0...0xFFE6, 0x1F300...0x1F64F, 0x1F680...0x1F6FF, 0x1F900...0x1FAFF, 0x20000...0x3FFFD:
            return 2
        case 0x200B...0x200F, 0x0300...0x036F, 0xFE00...0xFE0F: return 0
        default: return 1
        }
    }
}

struct MockScreen: Sendable {
    private(set) var cols: Int
    private(set) var rows: Int
    private(set) var cells: [PackedCell]
    private var dirty: [Bool]
    private var everythingDirty = true
    private var history: [[PackedCell]] = []
    private let historyLimit = 2000
    private(set) var displayOffset = 0

    var cursorRow = 0
    var cursorColumn = 0
    var cursorVisible = true
    var cursorShape: CursorShape = .block
    var cursorBlinking = false
    /// What `write` paints with.
    var pen = PackedCell()

    init(cols: Int, rows: Int) {
        self.cols = cols
        self.rows = rows
        cells = Array(repeating: PackedCell(), count: cols * rows)
        dirty = Array(repeating: true, count: rows)
    }

    // MARK: cells

    mutating func resize(cols: Int, rows: Int) {
        guard cols != self.cols || rows != self.rows else { return }
        // The mock does not reflow: it keeps what fits and starts the rest blank, which is all the
        // renderer needs to see (a full frame of a new size).
        var next = Array(repeating: PackedCell(), count: cols * rows)
        for r in 0..<min(rows, self.rows) {
            for c in 0..<min(cols, self.cols) { next[r * cols + c] = cells[r * self.cols + c] }
        }
        cells = next
        self.cols = cols
        self.rows = rows
        dirty = Array(repeating: true, count: rows)
        everythingDirty = true
        history.removeAll()
        displayOffset = 0
        cursorRow = min(cursorRow, rows - 1)
        cursorColumn = min(cursorColumn, cols - 1)
    }

    subscript(row: Int, column: Int) -> PackedCell {
        get { cells[row * cols + column] }
        set {
            let index = row * cols + column
            if cells[index] != newValue {
                cells[index] = newValue
                dirty[row] = true
            }
        }
    }

    /// Paint `scalar` at a cell with the pen's colours and flags `extra`; a double-width character takes two.
    mutating func put(_ scalar: UInt32, row: Int, column: Int, fg: UInt32? = nil, bg: UInt32? = nil, flags: CellFlags = []) {
        guard row >= 0, row < rows, column >= 0, column < cols else { return }
        var cell = pen
        if let fg { cell.fg = fg }
        if let bg { cell.bg = bg }
        cell.flags = pen.flags | flags.rawValue
        cell.scalar = scalar
        if CellWidth.of(scalar) == 2, column + 1 < cols {
            cell.flags |= CellFlags.wideChar.rawValue
            self[row, column] = cell
            var spacer = cell
            spacer.scalar = 0x20
            spacer.flags = (cell.flags & ~CellFlags.wideChar.rawValue) | CellFlags.wideCharSpacer.rawValue
            self[row, column + 1] = spacer
        } else {
            self[row, column] = cell
        }
    }

    mutating func clearRow(_ row: Int, bg: UInt32 = PackedColor.background) {
        for c in 0..<cols { self[row, c] = PackedCell(bg: bg) }
    }

    // MARK: text

    /// Write text at the cursor, wrapping and scrolling the region `top...bottom` at its end.
    mutating func write(_ text: String, top: Int = 0, bottom: Int? = nil) {
        let bottom = bottom ?? rows - 1
        for scalar in text.unicodeScalars {
            switch scalar.value {
            case 0x0A:
                newline(top: top, bottom: bottom)
            case 0x0D:
                cursorColumn = 0
            case 0x08, 0x7F:
                if cursorColumn > 0 {
                    cursorColumn -= 1
                    self[cursorRow, cursorColumn] = PackedCell(fg: pen.fg, bg: pen.bg)
                }
            case 0x00..<0x20:
                break
            default:
                let width = CellWidth.of(scalar.value)
                if width == 0 { continue }
                if cursorColumn + width > cols {
                    cursorColumn = 0
                    newline(top: top, bottom: bottom)
                }
                put(scalar.value, row: cursorRow, column: cursorColumn)
                cursorColumn += width
            }
        }
    }

    private mutating func newline(top: Int, bottom: Int) {
        if cursorRow == bottom {
            scrollUp(top: top, bottom: bottom)
        } else {
            cursorRow = min(rows - 1, cursorRow + 1)
        }
    }

    /// Move the rows `top...bottom` up one; the top row goes to the history when the region starts at the
    /// top of the screen. Every row of the region is damaged: the content under them all moved.
    mutating func scrollUp(top: Int, bottom: Int) {
        if top == 0 {
            history.append(Array(cells[0..<cols]))
            if history.count > historyLimit { history.removeFirst(history.count - historyLimit) }
            if displayOffset > 0 { displayOffset = min(displayOffset + 1, history.count) }
        }
        for r in top..<bottom {
            let from = (r + 1) * cols
            cells.withUnsafeMutableBufferPointer { buffer in
                for c in 0..<cols { buffer[r * cols + c] = buffer[from + c] }
            }
            dirty[r] = true
        }
        let blank = PackedCell(fg: pen.fg, bg: PackedColor.background)
        for c in 0..<cols { cells[bottom * cols + c] = blank }
        dirty[bottom] = true
    }

    mutating func scrollView(lines: Int) {
        let next = max(0, min(history.count, displayOffset + lines))
        if next != displayOffset {
            displayOffset = next
            everythingDirty = true
        }
    }

    mutating func snapToLive() {
        if displayOffset != 0 {
            displayOffset = 0
            everythingDirty = true
        }
    }

    // MARK: frames

    /// The cells of the visible row `row`: from the history when the view is scrolled back.
    private func visibleCells(_ row: Int) -> ArraySlice<PackedCell> {
        let virtual = history.count - displayOffset + row
        if virtual < history.count { return history[virtual][...] }
        let live = virtual - history.count
        return cells[live * cols..<(live + 1) * cols]
    }

    mutating func takeFrame(all: Bool) -> Data {
        // A scrolled view is always sent whole: which rows changed under it is not worth tracking.
        let full = all || everythingDirty || displayOffset > 0
        let changed = full ? rows : dirty.filter { $0 }.count
        let cursor: FrameHeader.Cursor? =
            cursorVisible && displayOffset == 0
            ? FrameHeader.Cursor(
                row: cursorRow, column: cursorColumn, shape: cursorShape,
                wide: cells[cursorRow * cols + cursorColumn].cellFlags.contains(.wideChar), blinking: cursorBlinking)
            : nil
        var writer = FrameWriter(
            header: FrameHeader(
                cols: cols, rows: rows, changedRows: changed, isFull: full, cursor: cursor,
                displayOffset: displayOffset, historySize: history.count),
            rowCapacity: changed)
        for r in 0..<rows where full || dirty[r] {
            visibleCells(r).withUnsafeBufferPointer { writer.appendRow(index: r, size: .single, cells: $0) }
        }
        for r in 0..<rows { dirty[r] = false }
        everythingDirty = false
        return writer.bytes
    }

    /// Whether anything is waiting to be sent (the header's cursor is compared by the renderer).
    var hasDamage: Bool { everythingDirty || dirty.contains(true) }
}
