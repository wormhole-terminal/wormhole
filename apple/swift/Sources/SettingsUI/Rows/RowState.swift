// What a row says about its value and where it is shown, apart from the control itself.

struct RowState {
    /// Differs from the default: a dot before the label and the reset arrow.
    var isChanged = false
    /// The reset arrow is always there, at the right end (the Changed page), not only under the pointer.
    var resetAlways = false
    /// The row's config key after the label (Advanced → Show config keys, or ⌥ held).
    var configKey: String?
    var found: Range<Int>?
    var foundDescription: Range<Int>?
    /// The row a search result jumped to: marked for a moment.
    var isRevealed = false
    /// The group's name after the label, in a results list where rows of different groups sit together.
    var context: String?
}
