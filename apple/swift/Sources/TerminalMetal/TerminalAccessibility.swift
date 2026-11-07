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
}
