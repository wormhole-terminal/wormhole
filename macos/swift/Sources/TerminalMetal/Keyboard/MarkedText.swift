// The text an input method is composing, and the questions the system asks about it. `NSTextInputClient`
// speaks UTF-16 ranges of a virtual document whose only content is this text (the terminal has no text
// storage); the renderer draws it in cells at the cursor. Pure, so it is tested without a window.

import Foundation
import TerminalCore

struct MarkedTextModel: Equatable {
    private(set) var text = ""
    /// The caret and any converted clause inside `text`, as UTF-16 offsets (what `setMarkedText` gives).
    private(set) var selection = NSRange(location: 0, length: 0)

    var isActive: Bool { !text.isEmpty }

    mutating func set(_ string: String, selection: NSRange) {
        text = string
        let length = string.utf16.count
        let location = selection.location == NSNotFound ? length : min(selection.location, length)
        self.selection = NSRange(location: location, length: min(selection.length, length - location))
    }

    mutating func clear() { self = MarkedTextModel() }

    var markedRange: NSRange { isActive ? NSRange(location: 0, length: text.utf16.count) : NSRange(location: NSNotFound, length: 0) }

    /// With nothing composed the selection is an empty range at 0: dictation and the emoji picker want a valid one.
    var selectedRange: NSRange { isActive ? selection : NSRange(location: 0, length: 0) }

    /// The part of the composed text in `range`, clamped (what reconversion and dictation ask for).
    func substring(in range: NSRange) -> (text: String, actual: NSRange)? {
        guard isActive else { return nil }
        let all = NSRange(location: 0, length: text.utf16.count)
        let actual = NSIntersectionRange(range, all)
        guard actual.length > 0 || range.length == 0, let swiftRange = Range(actual, in: text) else { return nil }
        return (String(text[swiftRange]), actual)
    }

    /// Characters (not UTF-16 units) before `offset`: what the renderer counts.
    func scalars(before offset: Int) -> Int {
        String(text.utf16.prefix(max(0, offset)))?.unicodeScalars.count ?? text.unicodeScalars.count
    }

    /// The cells the composed text covers before UTF-16 offset `offset`, where the candidate window should sit.
    func columns(before offset: Int) -> Int {
        guard let prefix = String(text.utf16.prefix(max(0, offset))) else { return 0 }
        return prefix.unicodeScalars.reduce(0) { $0 + CellWidth.of($1.value) }
    }

    /// What the renderer draws: the text, the caret and the converted clause in characters.
    var drawn: MarkedText? {
        guard isActive else { return nil }
        let caret = scalars(before: selection.location)
        let end = scalars(before: selection.location + selection.length)
        return MarkedText(text: text, caret: caret, clause: end - caret)
    }
}
