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
    /// How many characters from the caret are the clause being converted (drawn as a selection); 0: none.
    var clause = 0
}

struct SceneCounters {
    var frames = 0
    var rowsRebuilt = 0
    var cellsRead = 0
}

final class TerminalScene {
    /// What the rows are built with: swapped as a whole when the font changes (`setText`).
    private(set) var text: TextContext
    var fonts: FontSet { text.fonts }
    var atlas: GlyphAtlas { text.atlas }
    var words: WordCache { text.words }

    private(set) var columns = 0
    private(set) var rows = 0
    private var grid = UnsafeMutablePointer<PackedCell>.allocate(capacity: 0)
    private(set) var geometry: [RowGeometry] = []
    /// What each row's cells carry beyond a character (nil: nothing), from the last frame that named the row.
    private var extras: [[Int: CellExtra]?] = []
    /// How each row is drawn, from the last frame that named it.
    private(set) var sizes: [LineSize] = []
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
    /// The selection and the hovered link, per row; setting one rebuilds only the rows it changed on.
    private var marks: [RowMarks] = []
    /// What marks are painted with: the selection's and find's colours.
    private var palette: TerminalPalette

    init(text: TextContext, palette: TerminalPalette) {
        self.text = text
        self.palette = palette
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
            try FrameReader.forEachRow(bytes, header: header) { row, size, cells in
                (grid + row * columns).update(from: cells.baseAddress!, count: columns)
                sizes[row] = size
                extras[row] = nil
                dirty[row] = true
                counters.cellsRead += columns
            }
            for (row, found) in try CellExtras.read(bytes, header: header) where row < rows { extras[row] = found }
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
        sizes = Array(repeating: .single, count: rows)
        extras = Array(repeating: nil, count: rows)
        marks = Array(repeating: .none, count: rows)
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

    /// The selected columns on each row (empty: none). Rows whose marks changed are rebuilt.
    func setSelection(_ spans: [RowSpan]) { update(\.selected, spans) }

    /// The columns of the link under the pointer.
    func setLink(_ spans: [RowSpan]) { update(\.link, spans) }

    /// The colours marks are painted with: a change rebuilds the rows that carry a mark (find's tint is baked into
    /// the row; the selection's colours are slots, but whether the text keeps its own colour is not).
    func setPalette(_ value: TerminalPalette) {
        guard value != palette else { return }
        palette = value
        for row in marks.indices where marks[row].isMarked { dirty[row] = true }
    }

    /// Find's matches on each viewport row.
    func setFound(_ spans: [FindSpan]) {
        var next = [Int: [FoundColumns]]()
        for span in spans where span.row < rows && span.first <= span.last {
            next[span.row, default: []].append(FoundColumns(first: span.first, last: span.last, isCurrent: span.isCurrent))
        }
        for row in 0..<rows {
            let columns = next[row]?.sorted { $0.first < $1.first } ?? []
            if marks[row].found != columns {
                marks[row].found = columns
                dirty[row] = true
            }
        }
    }

    /// Start building with another font set: every row is built again from the cells already held, so the screen
    /// never shows a gap between the old text and the new. The atlas of the new context is its own.
    func setText(_ context: TextContext) {
        guard context !== text else { return }
        text = context
        markAllDirty()
        rebuildDirty()
        touch()
    }

    private func update(_ field: WritableKeyPath<RowMarks, ClosedRange<Int>?>, _ spans: [RowSpan]) {
        var next = [Int: ClosedRange<Int>]()
        for span in spans where span.row < rows && span.first <= span.last { next[span.row] = span.first...span.last }
        for row in 0..<rows where marks[row][keyPath: field] != next[row] {
            marks[row][keyPath: field] = next[row]
            dirty[row] = true
        }
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
        let composing = marked != nil && cursor?.row == row && marked?.text.isEmpty == false
        // The common row goes to the builder as it is; a row with a selection, a hovered link or composed text
        // is copied and marked first, so the hot loop knows none of it.
        let builder = text.builder, size = sizes[row]
        guard composing || marks[row].isMarked else {
            builder.build(
                row: row, size: size, cells: UnsafeBufferPointer(start: start, count: columns), extras: extras[row], into: geometry[row],
                cursor: draw)
            return
        }
        var cells = Array(UnsafeBufferPointer(start: start, count: columns))
        marks[row].apply(to: &cells, palette: palette)
        guard composing, let marked, let cursor else {
            cells.withUnsafeBufferPointer { builder.build(row: row, size: size, cells: $0, extras: extras[row], into: geometry[row], cursor: draw) }
            return
        }
        // Composed text replaces the cells from the cursor on, underlined, and the caret moves to its end.
        var column = cursor.column
        var caretColumn = column
        for (offset, scalar) in marked.text.unicodeScalars.enumerated() {
            let width = CellWidth.of(scalar.value)
            guard width > 0, column + width <= columns else { continue }
            var cell = PackedCell(scalar: scalar.value, flags: .underline)
            if offset >= marked.caret, offset < marked.caret + marked.clause { cell.bg = PackedColor.selectionBackground }
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
        cells.withUnsafeBufferPointer { builder.build(row: row, size: size, cells: $0, extras: extras[row], into: geometry[row], cursor: draw) }
    }

    // MARK: handing instances to the GPU

    /// Some row holds blinking text (SGR 5, 6): the view's blink clock runs only then.
    var hasBlinkingText: Bool { geometry.contains { $0.blinks } }

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
            result.append(cell.scalar == PackedCell.imagePlaceholder ? " " : Unicode.Scalar(cell.scalar) ?? " ")
            for mark in extras[row]?[column]?.marks ?? [] { result.append(Unicode.Scalar(mark) ?? " ") }
        }
        return String(result)
    }
}
