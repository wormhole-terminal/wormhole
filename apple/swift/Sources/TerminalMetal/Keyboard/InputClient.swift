// The input method: `NSTextInputClient`, which GPUI's key-only events never offered. It lets an input method
// compose Japanese, Chinese and Korean, dead keys make accents (Option-E then E), the emoji picker and
// Dictation insert text, and the candidate window sit at the cursor. The terminal has no text storage, so the
// document the system sees is the composed text alone (`MarkedTextModel`); committed text goes to the program
// as typed (`type`), except an ordinary typed character, which stays a key press so the engine encodes it by
// the modes the program set (kitty keyboard, modifiers).

import AppKit
import TerminalCore

extension MetalTerminalView: @preconcurrency NSTextInputClient {
    public func insertText(_ string: Any, replacementRange: NSRange) {
        handledByInputMethod = true
        let text = (string as? NSAttributedString)?.string ?? (string as? String) ?? ""
        clearComposition()
        guard !text.isEmpty else { return }
        if let key = pendingKey, key.text == text {
            send(key)
        } else {
            type(text)
        }
    }

    public override func doCommand(by selector: Selector) {
        handledByInputMethod = true
        // A key the input system maps to an editing command (arrows, return, backspace): the engine takes the
        // key itself, which also covers Escape cancelling nothing and Tab moving nowhere.
        if let key = pendingKey { send(key) }
    }

    public func setMarkedText(_ string: Any, selectedRange: NSRange, replacementRange: NSRange) {
        handledByInputMethod = true
        composition.set((string as? NSAttributedString)?.string ?? (string as? String) ?? "", selection: selectedRange)
        renderer.setMarked(composition.drawn)
        inputContext?.invalidateCharacterCoordinates()
        requestRedraw()
    }

    public func unmarkText() { clearComposition() }

    /// Drop what is being composed without committing it (a click, the window losing the keyboard).
    func endComposition() {
        guard composition.isActive else { return }
        inputContext?.discardMarkedText()
        clearComposition()
    }

    private func clearComposition() {
        guard composition.isActive else { return }
        composition.clear()
        renderer.setMarked(nil)
        requestRedraw()
    }

    public func hasMarkedText() -> Bool { composition.isActive }
    public func markedRange() -> NSRange { composition.markedRange }
    public func selectedRange() -> NSRange { composition.selectedRange }

    public func attributedSubstring(forProposedRange range: NSRange, actualRange: NSRangePointer?) -> NSAttributedString? {
        guard let part = composition.substring(in: range) else { return nil }
        actualRange?.pointee = part.actual
        return NSAttributedString(string: part.text)
    }

    /// Everything is underlined already: the input method has no attributes to add.
    public func validAttributesForMarkedText() -> [NSAttributedString.Key] { [] }

    /// Where the candidate window opens: under the composed text's start, or the cursor's cell, in screen coordinates.
    public func firstRect(forCharacterRange range: NSRange, actualRange: NSRangePointer?) -> NSRect {
        let grid = cellGrid
        let position = renderer.cursorCell ?? (0, 0)
        let column = position.column + composition.columns(before: range.location)
        let local = NSRect(x: CGFloat(column) * grid.cellWidth, y: CGFloat(position.row) * grid.cellHeight, width: grid.cellWidth, height: grid.cellHeight)
        guard let window else { return local }
        return window.convertToScreen(convert(local, to: nil))
    }

    public func characterIndex(for point: NSPoint) -> Int { 0 }
}
