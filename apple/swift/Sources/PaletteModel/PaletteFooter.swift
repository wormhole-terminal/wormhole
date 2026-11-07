// The footer's words: the keys that work on what is selected, and a count or a hint at the right (core's
// `command_palette/view.rs` `render_footer`). ⌘↩ (open in a new window) is left out: this app has one window.

import Foundation

public struct KeyHint: Sendable, Equatable {
    public var keys: String
    public var label: String
}

public enum PaletteFooter {
    public static func hints(scope: PaletteScope) -> [KeyHint] {
        let pairs: [(String, String)] =
            switch scope {
            case .terminals: [("↩", "switch"), ("⌘⌫", "close terminal")]
            case .actions: [("↩", "run"), ("⌫", "back to all")]
            default: [("↑↓", "move"), ("↩", "open"), ("tab", "scope")]
            }
        return pairs.map { KeyHint(keys: $0.0, label: $0.1) }
    }

    /// "3 results" once something is typed; with all searched and nothing typed, how to reach the actions.
    public enum Trailing: Sendable, Equatable {
        case count(Int)
        case actionsHint
        case none
    }

    public static func trailing(scope: PaletteScope, query: String, rows: [PaletteRow]) -> Trailing {
        if !query.trimmingCharacters(in: .whitespaces).isEmpty { return .count(PaletteRows.resultCount(rows)) }
        return scope == .all ? .actionsHint : .none
    }
}
