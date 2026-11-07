// Keyboard, paste, scroll and the input method. A key goes through `interpretKeyEvents` (so an input
// method can compose Japanese, accents, emoji) unless it is a control chord, a key that types nothing, or
// option-as-meta: those go to the engine directly. AppKit then calls back: `insertText` for committed text,
// `setMarkedText` while composing, `doCommand` for the keys that are commands (return, arrows, backspace).

import AppKit
import TerminalCore

extension MetalTerminalView {
    // MARK: keys

    public override func keyDown(with event: NSEvent) {
        guard let key = KeyMap.input(from: event, leftOptionAsMeta: leftOptionAsMeta), !key.platform else {
            super.keyDown(with: event)
            return
        }
        if !hasMarkedText(), key.control || KeyMap.isNonText(key) || (key.alt && key.optionAsMeta) {
            send(key)
            return
        }
        pendingKey = key
        handledByInputMethod = false
        interpretKeyEvents([event])
        pendingKey = nil
        // Nothing consumed it (a key the input system has no opinion about): the engine takes it as it is.
        if !handledByInputMethod { send(key) }
    }

    func send(_ key: KeyInput) {
        if source.encodesKeys {
            _ = source.sendKey(key)
        } else if let bytes = KeyEncoder.encode(key) {
            source.write(bytes)
        }
        // Typing returns the view to the live screen on the source's side; nothing to do here.
    }

    public override func performKeyEquivalent(with event: NSEvent) -> Bool {
        // The app has no menu bar in a test instance, so its shortcuts are caught here.
        if event.modifierFlags.intersection(.deviceIndependentFlagsMask) == .command, event.charactersIgnoringModifiers == "v" {
            paste(nil)
            return true
        }
        return super.performKeyEquivalent(with: event)
    }

    // MARK: paste

    @objc public func paste(_ sender: Any?) {
        guard let text = NSPasteboard.general.string(forType: .string), !text.isEmpty else { return }
        source.paste(text)
    }

    // MARK: mouse

    public override func mouseDown(with event: NSEvent) {
        window?.makeFirstResponder(self)
    }

    public override func scrollWheel(with event: NSEvent) {
        let cellHeight = CGFloat(renderer.cellPixels.height) / CGFloat(renderer.scale)
        // A wheel reports lines, a trackpad points.
        let points = event.hasPreciseScrollingDeltas ? event.scrollingDeltaY : event.scrollingDeltaY * cellHeight
        scrollRemainder += points
        let lines = Int((scrollRemainder / cellHeight).rounded(.towardZero))
        scrollRemainder -= CGFloat(lines) * cellHeight
        if lines != 0 { source.scroll(lines: lines) }
    }
}

// MARK: the input method

extension MetalTerminalView: @preconcurrency NSTextInputClient {
    public func insertText(_ string: Any, replacementRange: NSRange) {
        handledByInputMethod = true
        let text = (string as? NSAttributedString)?.string ?? (string as? String) ?? ""
        clearMarked()
        guard !text.isEmpty else { return }
        if let key = pendingKey, key.text == text {
            // An ordinary typed character keeps the engine's key encoding (kitty keyboard, modifiers).
            send(key)
        } else {
            // A composed commit or a dead-key result: text, not a key.
            source.write(Data(text.utf8))
        }
    }

    public override func doCommand(by selector: Selector) {
        handledByInputMethod = true
        if let key = pendingKey { send(key) }
    }

    public func setMarkedText(_ string: Any, selectedRange: NSRange, replacementRange: NSRange) {
        handledByInputMethod = true
        markedString = (string as? NSAttributedString)?.string ?? (string as? String) ?? ""
        // The caret is a UTF-16 offset; the renderer counts characters.
        let prefix = String(markedString.utf16.prefix(max(0, selectedRange.location))) ?? ""
        markedCaret = selectedRange.location == NSNotFound ? markedString.unicodeScalars.count : prefix.unicodeScalars.count
        renderer.setMarked(text: markedString, caret: markedCaret)
        requestRedraw()
    }

    public func unmarkText() { clearMarked() }

    func clearMarked() {
        guard !markedString.isEmpty else { return }
        markedString = ""
        markedCaret = 0
        renderer.setMarked(text: nil)
        requestRedraw()
    }

    public func hasMarkedText() -> Bool { !markedString.isEmpty }

    public func markedRange() -> NSRange {
        markedString.isEmpty ? NSRange(location: NSNotFound, length: 0) : NSRange(location: 0, length: markedString.utf16.count)
    }

    public func selectedRange() -> NSRange {
        markedString.isEmpty ? NSRange(location: NSNotFound, length: 0) : NSRange(location: markedString.utf16.count, length: 0)
    }

    public func attributedSubstring(forProposedRange range: NSRange, actualRange: NSRangePointer?) -> NSAttributedString? { nil }

    public func validAttributesForMarkedText() -> [NSAttributedString.Key] { [] }

    /// Where the candidate window opens: the cursor's cell, in screen coordinates.
    public func firstRect(forCharacterRange range: NSRange, actualRange: NSRangePointer?) -> NSRect {
        let cell = renderer.cellPixels
        let scale = CGFloat(renderer.scale)
        let position = renderer.cursorCell ?? (0, 0)
        let local = NSRect(
            x: CGFloat(position.column * cell.width) / scale, y: CGFloat(position.row * cell.height) / scale,
            width: CGFloat(cell.width) / scale, height: CGFloat(cell.height) / scale)
        guard let window else { return local }
        return window.convertToScreen(convert(local, to: nil))
    }

    public func characterIndex(for point: NSPoint) -> Int { 0 }
}
