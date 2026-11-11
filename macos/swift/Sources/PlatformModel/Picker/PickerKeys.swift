// What the picker's keys do, as one pure rule (`view/picker/keys.rs`), so the panel that owns the keyboard can
// ask on the spot whether an arrow is the picker's or the field's.

import Foundation

public enum PickerKey: Sendable, Equatable {
    case up, down, pageUp, pageDown, enter, tab, right, left, escape
    /// ⌘⌫ on the selected row.
    case forget
}

/// The text field as it stands: its text and where the caret is, as character offsets.
public struct PickerField: Sendable, Equatable {
    public var text: String
    public var caret: Int
    /// How many characters are selected.
    public var selected: Int

    public init(text: String, caret: Int? = nil, selected: Int = 0) {
        self.text = text
        self.caret = caret ?? text.count
        self.selected = selected
    }

    var caretAtEnd: Bool { selected == 0 && caret >= text.count }
    var caretAtStart: Bool { selected == 0 && caret == 0 }
}

/// What the model says about itself that the keys depend on.
public struct PickerFacts: Sendable, Equatable {
    public var up = false
    public var upAtEnd = false
    /// The selected row can be gone into.
    public var descends = false
    public var ghost = false
    public var forgettable = false
    public init() {}
}

public enum PickerDo: Sendable, Equatable {
    /// Not the picker's: the field has it (an arrow moves the caret).
    case pass
    case close
    case step(Int)
    case pick
    case descend
    case up
    case forget
}

public enum PickerKeys {
    /// How far a page key moves the selection.
    public static let page = 8

    /// ↑ ↓ and the page keys move the selection, ↩ picks, ⎋ closes, ⇥ goes into the row; → only with the caret at
    /// the end and something to go into; ← only at the very start (or at the end where the text ends a level).
    public static func decide(_ key: PickerKey, field: PickerField, facts: PickerFacts) -> PickerDo {
        switch key {
        case .escape: .close
        case .up: .step(-1)
        case .down: .step(1)
        case .pageUp: .step(-page)
        case .pageDown: .step(page)
        case .enter: .pick
        case .tab: .descend
        case .right where field.caretAtEnd && (facts.ghost || facts.descends): .descend
        case .left where facts.up && (field.caretAtStart || (field.caretAtEnd && facts.upAtEnd)): .up
        case .forget where facts.forgettable: .forget
        case .right, .left, .forget: .pass
        }
    }

    /// The grey completion after the caret, as fish draws it: what follows `typed` in `completion`, when
    /// `completion` begins with it (ignoring case) and goes on past it (`view/picker/ghost.rs`).
    public static func ghost(typed: String, completion: String) -> String? {
        guard !typed.isEmpty, completion.count > typed.count,
            completion.prefix(typed.count).compare(typed, options: .caseInsensitive) == .orderedSame
        else { return nil }
        return String(completion.dropFirst(typed.count))
    }
}
