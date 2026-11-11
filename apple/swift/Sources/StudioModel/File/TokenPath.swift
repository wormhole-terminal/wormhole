// Where a token's own value lives inside a theme file: the one place that knows a UI token is `ui[name]`, a
// terminal colour `terminal[key]`, a palette slot `terminal.ansi[n]`, a syntax colour `syntax[key]` and a
// metric `metrics[key]` (themes.md "Format"; `studio/document/layers.rs` in the GPUI app).

import Foundation

enum TokenPath: Equatable {
    case ui(String)
    case terminal(String)
    case ansi(Int)
    case syntax(String)
    case metric(String)

    init(_ token: String) {
        if let key = token.dropPrefix("metrics.") {
            self = .metric(key)
        } else if let slot = token.dropPrefix("terminal.ansi."), let index = Int(slot), (0..<16).contains(index) {
            self = .ansi(index)
        } else if let key = token.dropPrefix("terminal.") {
            self = .terminal(key)
        } else if let key = token.dropPrefix("syntax.") {
            self = .syntax(key)
        } else {
            self = .ui(token)
        }
    }

    /// The object in a layer that holds the value, and its key there.
    var location: (section: String, key: String) {
        switch self {
        case .ui(let name): ("ui", name)
        case .terminal(let key): ("terminal", key)
        case .ansi(let slot): ("terminal.ansi", String(slot))
        case .syntax(let key): ("syntax", key)
        case .metric(let key): ("metrics", key)
        }
    }

    /// Shape and syntax are shared by a theme's appearances; so is the primary colour (`token_scope`).
    func isShared(group: String?) -> Bool {
        switch self {
        case .metric, .syntax: true
        default: group == "Primary"  // text: a token group's name as core's catalog spells it, a key and not a label
        }
    }
}

extension String {
    func dropPrefix(_ prefix: String) -> String? { hasPrefix(prefix) ? String(dropFirst(prefix.count)) : nil }
}
