// What the renderer knows between frames: the cells of every row, the instances built from them, and
// the cursor. A frame names the rows that changed; only those (and the rows the cursor, focus or
// composed text touch) are rebuilt, and the GPU buffers are filled from every row's kept instances.

import Foundation
import Metal
import TerminalCore

/// Text being composed by an input method, drawn at the cursor with an underline until it is committed.
struct MarkedText: Equatable {
    var text: String
    /// The caret's offset in `text`, in characters.
    var caret: Int
}

struct SceneCounters {
    var frames = 0
    var rowsRebuilt = 0
    var cellsRead = 0
}

final class TerminalScene {
    let fonts: FontSet
    let atlas: GlyphAtlas
    let words: WordCache
    private let builder: RowBuilder

    private(set) var columns = 0
    private(set) var rows = 0
    private var grid = UnsafeMutablePointer<PackedCell>.allocate(capacity: 0)
    private(set) var geometry: [RowGeometry] = []
    private var dirty: [Bool] = []
    private(set) var counters = SceneCounters()
    /// Bumped whenever the instances changed, so the renderer knows whether a draw would show anything new.
    private(set) var changeCount = 0
    private(set) var cursor: FrameHeader.Cursor?
    private(set) var header: FrameHeader?

    /// The blink phase and whether the window is key: both only change which rows are rebuilt.
    private(set) var cursorOn = true
    private(set) var focused = true
    private(set) var marked: MarkedText?

    init(fonts: FontSet, device: MTLDevice) {
        self.fonts = fonts
        atlas = GlyphAtlas(device: device)
        words = WordCache(fonts: fonts, atlas: atlas)
        builder = RowBuilder(fonts: fonts, atlas: atlas, words: words)
    }

    deinit { grid.deallocate() }

    // MARK: frames

    /// Take a packed frame and rebuild what it changed. Returns how many rows were rebuilt.
    @discardableResult
    func apply(_ data: Data) throws -> Int {
        guard !data.isEmpty else { return 0 }
        counters.frames += 1
        try data.withUnsafeBytes { bytes in
            let header = try FrameReader.header(bytes)
            if header.cols != columns || header.rows != rows { resize(columns: header.cols, rows: header.rows) }
            self.header = header
            try FrameReader.forEachRow(bytes, header: header) { row, _, cells in
                (grid + row * columns).update(from: cells.baseAddress!, count: columns)
                dirty[row] = true
                counters.cellsRead += columns
            }
            setCursor(header.cursor)
        }
        return rebuildDirty()
    }

    private func resize(columns: Int, rows: Int) {
        grid.deallocate()
        grid = .allocate(capacity: max(1, columns * rows))
        grid.initialize(repeating: PackedCell(), count: max(1, columns * rows))
        self.columns = columns
        self.rows = rows
        geometry = (0..<rows).map { _ in RowGeometry() }
        dirty = Array(repeating: true, count: rows)
        cursor = nil
    }

    private func setCursor(_ next: FrameHeader.Cursor?) {
        guard next != cursor else { return }
        if let old = cursor, old.row < rows { dirty[old.row] = true }
        if let next, next.row < rows { dirty[next.row] = true }
        cursor = next
    }

    func setCursorOn(_ on: Bool) {
        guard on != cursorOn else { return }
        cursorOn = on
        markCursorRow()
    }

    func setFocused(_ value: Bool) {
        guard value != focused else { return }
        focused = value
        markCursorRow()
    }

    func setMarked(_ value: MarkedText?) {
        guard value != marked else { return }
        marked = value
        markCursorRow()
    }

    private func markCursorRow() {
        if let cursor, cursor.row < rows { dirty[cursor.row] = true }
    }

    /// Something the instances do not hold changed (the palette): draw again without rebuilding.
    func touch() { changeCount += 1 }

    func markAllDirty() {
        for index in dirty.indices { dirty[index] = true }
    }

    var hasDirtyRows: Bool { dirty.contains(true) }

    // MARK: building

    @discardableResult
    func rebuildDirty() -> Int {
        var built = 0
        let generation = atlas.generation
        for row in 0..<rows where dirty[row] {
            build(row)
            built += 1
        }
        // The atlas started over while rows were built: instances made before it point at glyphs that are
        // gone. Rebuild everything once (the atlas now holds what this screen uses).
        if atlas.generation != generation {
            for row in 0..<rows {
                build(row)
                built += 1
            }
        }
        counters.rowsRebuilt += built
        if built > 0 { changeCount += 1 }
        return built
    }

    private func build(_ row: Int) {
        dirty[row] = false
        let start = grid + row * columns
        var draw: CursorDraw?
        if let cursor, cursor.row == row, cursorOn || !cursor.blinking {
            draw = CursorDraw(column: cursor.column, shape: cursor.shape, wide: cursor.wide, focused: focused)
        }
        guard let marked, let cursor, cursor.row == row, !marked.text.isEmpty else {
            builder.build(row: row, cells: UnsafeBufferPointer(start: start, count: columns), into: geometry[row], cursor: draw)
            return
        }
        // Composed text replaces the cells from the cursor on, underlined, and the caret moves to its end.
        var cells = Array(UnsafeBufferPointer(start: start, count: columns))
        var column = cursor.column
        var caretColumn = column
        for (offset, scalar) in marked.text.unicodeScalars.enumerated() {
            let width = CellWidth.of(scalar.value)
            guard width > 0, column + width <= columns else { continue }
            var cell = PackedCell(scalar: scalar.value, flags: .underline)
            if width == 2 {
                cell.flags |= CellFlags.wideChar.rawValue
                cells[column] = cell
                cell.scalar = 0x20
                cell.flags = CellFlags.underline.rawValue | CellFlags.wideCharSpacer.rawValue
                cells[column + 1] = cell
            } else {
                cells[column] = cell
            }
            column += width
            if offset < marked.caret { caretColumn = column }
        }
        draw?.column = min(caretColumn, columns - 1)
        draw?.shape = .beam
        cells.withUnsafeBufferPointer { builder.build(row: row, cells: $0, into: geometry[row], cursor: draw) }
    }

    // MARK: handing instances to the GPU

    var instanceCounts: (backgrounds: Int, decorations: Int, glyphs: Int) {
        var counts = (backgrounds: 0, decorations: 0, glyphs: 0)
        for row in geometry {
            counts.backgrounds += row.backgrounds.count
            counts.decorations += row.decorations.count
            counts.glyphs += row.glyphs.count
        }
        return counts
    }

    /// Copy every row's instances into buffers sized for `instanceCounts`: backgrounds then decorations
    /// into `solids`, glyphs into `glyphs`.
    func copyInstances(solids: UnsafeMutableRawPointer, glyphs: UnsafeMutableRawPointer, counts: (backgrounds: Int, decorations: Int, glyphs: Int)) {
        var background = solids
        var decoration = solids + counts.backgrounds * MemoryLayout<SolidInstance>.stride
        var glyph = glyphs
        for row in geometry {
            row.backgrounds.withUnsafeBytes { bytes in
                guard let base = bytes.baseAddress else { return }
                background.copyMemory(from: base, byteCount: bytes.count)
                background += bytes.count
            }
            row.decorations.withUnsafeBytes { bytes in
                guard let base = bytes.baseAddress else { return }
                decoration.copyMemory(from: base, byteCount: bytes.count)
                decoration += bytes.count
            }
            row.glyphs.withUnsafeBytes { bytes in
                guard let base = bytes.baseAddress else { return }
                glyph.copyMemory(from: base, byteCount: bytes.count)
                glyph += bytes.count
            }
        }
    }

    /// The text of a row as it sits in the grid (for tests, and the accessibility node later).
    func text(ofRow row: Int) -> String {
        guard row >= 0, row < rows else { return "" }
        var result = String.UnicodeScalarView()
        for column in 0..<columns {
            let cell = grid[row * columns + column]
            if cell.cellFlags.contains(.wideCharSpacer) { continue }
            result.append(Unicode.Scalar(cell.scalar) ?? " ")
        }
        return String(result)
    }
}
