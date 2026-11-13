// What the popover open now says, kept for the control folder (`popover`, `popover-press`, `popover-type`, `popover-key`,
// `popover-colour`, `popover-close`, `popover-native`, `popover-real`): a SwiftUI popover's controls are closures inside its view, which
// nothing outside can list or press, so a popover that a script should reach describes its rows here while it is open and says what
// pressing each does. GPUI's recorder does the same for its popovers. Nothing here draws or decides; the closures are the ones the
// view's own buttons run.

import AppKit
import SwiftUI

@MainActor
public enum PopoverRecord {
    /// One line of the popover.
    public struct Row {
        public enum Kind {
            case heading
            /// A text field and whether it has the keys.
            case field(text: String, focused: Bool)
            /// A row of colours by name, the chosen one's place, and whether a colour of the person's own is set.
            case colours(names: [String], selected: Int?, custom: Bool)
            /// A button or an action row: pressing it runs what a click does.
            case action
        }

        public var kind: Kind
        public var title: String
        public var press: (@MainActor () -> Void)?

        public init(_ kind: Kind, _ title: String, press: (@MainActor () -> Void)? = nil) { (self.kind, self.title, self.press) = (kind, title, press) }
    }

    /// An open popover: its rows, and what typing, a key, a colour, closing and drawing it do.
    public struct Open {
        public var rows: [Row]
        public var type: @MainActor (String) -> Void
        /// `enter` keeps what is typed and closes, `escape` closes; false for a key it has no use for.
        public var key: @MainActor (String) -> Bool
        /// A colour by name, or `#rrggbb`; false for one it does not know.
        public var colour: @MainActor (String) -> Bool
        public var close: @MainActor () -> Void
        /// The same content in a hosting view of its own, for drawing it to a picture or showing it in a popover of AppKit's own.
        public var host: @MainActor () -> NSView

        public init(
            rows: [Row], type: @escaping @MainActor (String) -> Void, key: @escaping @MainActor (String) -> Bool,
            colour: @escaping @MainActor (String) -> Bool, close: @escaping @MainActor () -> Void, host: @escaping @MainActor () -> NSView
        ) {
            (self.rows, self.type, self.key, self.colour, self.close, self.host) = (rows, type, key, colour, close, host)
        }
    }

    public private(set) static var open: Open?

    public static func set(_ open: Open?) { self.open = open }

    /// The rows one a line, as GPUI's `popover` prints them: `# heading`, `field "name" (focused)`, `colours: Grey | Blue [x] | … | custom`,
    /// `- action`.
    public static var listing: String {
        guard let open else { return "no popover" } // text: control state line
        return open.rows.map { row -> String in
            switch row.kind {
            case .heading: "# \(row.title)" // text: control state line
            case .field(let text, let focused): "field \(text.debugDescription) (\(row.title))\(focused ? " (focused)" : "")" // text: control state line
            case .colours(let names, let selected, let custom):
                "colours: " + (names.enumerated().map { $0.offset == selected ? "\($0.element) [x]" : $0.element } + ["custom\(custom ? " [x]" : "")"]).joined(separator: " | ") // text: control state line
            case .action: "- \(row.title)" // text: control state line
            }
        }.joined(separator: "\n")
    }
}
