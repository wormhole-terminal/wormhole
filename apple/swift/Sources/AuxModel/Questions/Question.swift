// A question the app asks before it does something that ends work: a native alert (a sheet on the window), so
// macOS draws it and it behaves as every Mac alert does (Return presses the first button, Esc cancels).

import Foundation

public struct Question: Equatable, Sendable {
    public enum Style: Sendable { case warning, info }

    public var title: String
    public var detail: String
    /// The confirming button, then buttons that also go ahead in another way ("Restart When Idle", "Leave It
    /// Running"), then the one that backs out and does nothing.
    public var confirm: String
    public var alsoButtons: [String]
    public var cancel: String
    /// A checkbox above the buttons, with its first state ("Keep terminals running in the background").
    public var checkbox: Checkbox?
    public var style: Style

    public struct Checkbox: Equatable, Sendable {
        public var label: String
        public var checked: Bool
        public init(label: String, checked: Bool) { (self.label, self.checked) = (label, checked) }
    }

    public init(
        title: String, detail: String, confirm: String, alsoButtons: [String] = [],
        cancel: String = AuxText.cancel, checkbox: Checkbox? = nil, style: Style = .warning
    ) {
        (self.title, self.detail, self.confirm, self.alsoButtons) = (title, detail, confirm, alsoButtons)
        (self.cancel, self.checkbox, self.style) = (cancel, checkbox, style)
    }
}

/// What the person chose: a button by its place (0 confirms, 1... the "also" buttons), or nothing, and the box.
public enum Answer: Equatable, Sendable {
    case confirm(boxChecked: Bool)
    case also(Int)
    case cancel
}
