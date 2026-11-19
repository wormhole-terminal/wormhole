// What core offers while a value is typed (`theme::tokens::completions`): the names and functions that fit the word at
// the caret, and the part of the text the one taken replaces. The ranges are UTF-8 offsets, as core counts bytes.

public struct FieldCompletion: Sendable, Equatable {
    /// What the list shows: the name, or the function with what it takes (`mix(a, b, 30%)`).
    public var label: String
    /// What takes the place of the word typed: the name, or the function and its `(`.
    public var insert: String
    public var function: Bool

    public init(label: String, insert: String, function: Bool) { (self.label, self.insert, self.function) = (label, insert, function) }
}

/// What a token's value field shows right now, as the field itself tells it (`ExpressionField`): its text and the list under
/// it (empty: none is up). The control folder's `studio-complete` reads this, never a list worked out beside the field.
public struct FieldOffer: Sendable, Equatable {
    public var token: String
    public var text: String
    public var items: [FieldCompletion]

    public init(token: String, text: String, items: [FieldCompletion]) { (self.token, self.text, self.items) = (token, text, items) }
}

public struct FieldCompletions: Sendable, Equatable {
    /// The UTF-8 offsets of the word the caret is in.
    public var range: Range<Int>
    public var items: [FieldCompletion]

    public init(range: Range<Int>, items: [FieldCompletion]) { (self.range, self.items) = (range, items) }

    /// `text` with the word replaced by `item`'s insert, and the UTF-8 offset the caret goes to (after it).
    public func taking(_ item: FieldCompletion, in text: String) -> (text: String, caret: Int) {
        var bytes = Array(text.utf8)
        bytes.replaceSubrange(range, with: Array(item.insert.utf8))
        return (String(decoding: bytes, as: UTF8.self), range.lowerBound + item.insert.utf8.count)
    }
}
