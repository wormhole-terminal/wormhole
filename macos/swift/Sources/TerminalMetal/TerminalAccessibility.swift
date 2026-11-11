// What VoiceOver gets from the terminal view: a text area labelled "Terminal" whose value is the screen as
// plain text (one line per row) and whose selected text is the selection. Asked for lazily, only when a
// screen reader is looking, never per frame.

import AppKit

extension MetalTerminalView {
    public override func isAccessibilityElement() -> Bool { true }
    public override func accessibilityRole() -> NSAccessibility.Role? { .textArea }
    public override func accessibilityLabel() -> String? { TerminalWord.accessibilityLabel.text }

    public override func accessibilityValue() -> Any? {
        (0..<renderer.gridSize.rows).map { renderer.text(ofRow: $0).trimmingCharacters(in: .whitespaces) }.joined(separator: "\n")
    }

    public override func accessibilitySelectedText() -> String? { interaction?.selectionText() }

    /// The screen's lines as `accessibilityValue` writes them, so the ranges below index the same string.
    private var accessibleLines: [String] {
        (0..<renderer.gridSize.rows).map { renderer.text(ofRow: $0).trimmingCharacters(in: .whitespaces) }
    }

    public override func accessibilityNumberOfCharacters() -> Int { accessibleLines.joined(separator: "\n").utf16.count }

    /// The line the cursor is on, so VoiceOver reads it when the cursor moves (GPUI's text node does the same).
    public override func accessibilityInsertionPointLineNumber() -> Int { renderer.cursorCell?.row ?? 0 }

    public override func accessibilityLine(for index: Int) -> Int {
        var start = 0
        for (row, line) in accessibleLines.enumerated() {
            start += line.utf16.count + 1
            if index < start { return row }
        }
        return max(accessibleLines.count - 1, 0)
    }

    public override func accessibilityRange(forLine line: Int) -> NSRange {
        let lines = accessibleLines
        guard lines.indices.contains(line) else { return NSRange(location: 0, length: 0) }
        let start = lines[..<line].reduce(0) { $0 + $1.utf16.count + 1 }
        return NSRange(location: start, length: lines[line].utf16.count)
    }

    /// The selection in the value, or the cursor as an empty range.
    public override func accessibilitySelectedTextRange() -> NSRange {
        let value = accessibleLines.joined(separator: "\n") as NSString
        if let selected = interaction?.selectionText(), !selected.isEmpty {
            let found = value.range(of: selected)
            if found.location != NSNotFound { return found }
        }
        let row = renderer.cursorCell?.row ?? 0
        let lines = accessibleLines
        guard lines.indices.contains(row) else { return NSRange(location: value.length, length: 0) }
        let column = min(renderer.cursorCell?.column ?? 0, lines[row].utf16.count)
        return NSRange(location: lines[..<row].reduce(0) { $0 + $1.utf16.count + 1 } + column, length: 0)
    }
}
