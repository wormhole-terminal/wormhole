// The mock's pointer, selection and links, over rows of text. Enough of the engine's behaviour to drive the
// view without Rust: a selection by cell, word, line or block, URLs found in the text, and a program that
// takes the mouse when told to (it then records what it was given). Not the engine: it knows nothing of
// wrapped lines, wide characters or the display offset.

import Foundation
import Synchronization

public final class MockInteraction: TerminalInteraction {
    private struct State {
        var kind = SelectionKind.cell
        var anchor: (row: Int, column: Int)?
        var head = (row: 0, column: 0)
        var program = false
        var reported: [PointerInput] = []
        var wheels: [WheelInput] = []
        var focus: [Bool] = []
    }

    private let rows: @Sendable () -> [String]
    private let state = Mutex(State())

    /// `rows` gives the visible rows' text, one character per column.
    public init(rows: @escaping @Sendable () -> [String]) { self.rows = rows }

    /// Whether a program has asked for the mouse; while it has, `reportPointer` takes every event.
    public var programTakesPointer: Bool {
        get { state.withLock { $0.program } }
        set { state.withLock { $0.program = newValue } }
    }
    public var reportedPointer: [PointerInput] { state.withLock { $0.reported } }
    public var wheels: [WheelInput] { state.withLock { $0.wheels } }
    public var focusChanges: [Bool] { state.withLock { $0.focus } }

    public var takesPointer: Bool { programTakesPointer }

    public func reportPointer(_ input: PointerInput) -> Bool {
        state.withLock {
            guard $0.program, !input.modifiers.shift else { return false }
            $0.reported.append(input)
            return true
        }
    }

    public func scrollWheel(_ input: WheelInput) { state.withLock { $0.wheels.append(input) } }
    public func focusChanged(_ focused: Bool) { state.withLock { $0.focus.append(focused) } }
    public func droppedText(paths: [String]) -> String { paths.map { $0.replacingOccurrences(of: " ", with: "\\ ") + " " }.joined() }
    public func clearHistory() -> Bool { false }

    // MARK: selection

    public func selectionStart(kind: SelectionKind, row: Int, column: Int, side: CellSide) {
        state.withLock {
            $0.kind = kind
            $0.anchor = (row, column)
            $0.head = (row, column)
        }
    }

    public func selectionUpdate(row: Int, column: Int, side: CellSide) { state.withLock { $0.head = (row, column) } }

    public func selectionFinish() {
        state.withLock { state in
            if state.kind == .cell, let anchor = state.anchor, anchor == state.head { state.anchor = nil }
        }
    }

    public func selectAll() {
        let text = rows()
        state.withLock {
            $0.kind = .cell
            $0.anchor = (0, 0)
            $0.head = (max(0, text.count - 1), max(0, (text.first?.count ?? 1) - 1))
        }
    }

    public func selectionSpans() -> [RowSpan] {
        let text = rows()
        let (kind, anchor, head) = state.withLock { ($0.kind, $0.anchor, $0.head) }
        guard let anchor, !text.isEmpty else { return [] }
        let width = text[0].count
        let (first, last) = anchor.row <= head.row ? (anchor, head) : (head, anchor)
        return (first.row...last.row).compactMap { row in
            guard row < text.count else { return nil }
            switch kind {
            case .line: return RowSpan(row: row, first: 0, last: width - 1)
            case .block:
                return RowSpan(row: row, first: min(anchor.column, head.column), last: max(anchor.column, head.column))
            case .word:
                let (from, to) = Self.word(in: text[row], at: row == first.row ? first.column : 0)
                return RowSpan(row: row, first: from, last: row == last.row ? max(to, Self.word(in: text[row], at: last.column).1) : width - 1)
            case .cell:
                return RowSpan(row: row, first: row == first.row ? first.column : 0, last: row == last.row ? last.column : width - 1)
            }
        }
    }

    public func selectionText() -> String? {
        let text = rows()
        let lines = selectionSpans().map { span -> String in
            let characters = Array(text[span.row])
            return String(characters[min(span.first, characters.count)..<min(span.last + 1, characters.count)]).trimmingCharacters(in: .whitespaces)
        }
        return lines.isEmpty ? nil : lines.joined(separator: "\n")
    }

    private static func word(in line: String, at column: Int) -> (Int, Int) {
        let characters = Array(line)
        guard column < characters.count, !characters[column].isWhitespace else { return (column, column) }
        var from = column, to = column
        while from > 0, !characters[from - 1].isWhitespace { from -= 1 }
        while to + 1 < characters.count, !characters[to + 1].isWhitespace { to += 1 }
        return (from, to)
    }

    // MARK: links

    public func link(row: Int, column: Int) -> LinkInfo? {
        let text = rows()
        guard row < text.count else { return nil }
        let line = text[row]
        let pattern = try? Regex("https?://[^\\s]+")
        for match in line.matches(of: pattern!) {
            let start = line.distance(from: line.startIndex, to: match.range.lowerBound)
            let end = line.distance(from: line.startIndex, to: match.range.upperBound) - 1
            if (start...end).contains(column) {
                return LinkInfo(uri: String(line[match.range]), isExplicit: false, spans: [RowSpan(row: row, first: start, last: end)])
            }
        }
        return nil
    }

    public func openTarget(for uri: String) -> OpenTarget { uri.hasPrefix("http") ? .url(uri) : .refused }
}
