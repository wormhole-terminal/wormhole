// What a token is when the catalogue has no line for it: a palette slot, a syntax colour, the terminal's own
// colours (`describe` in `studio/token_editor.rs`).

import Foundation

enum StudioDescriptions {
    static func describe(_ token: String) -> String {
        if let slot = token.dropPrefix("terminal.ansi.") { return String(localized: "studio-describe-ansi", defaultValue: "Terminal palette color \(slot)") }
        if let key = token.dropPrefix("syntax.") { return String(localized: "studio-describe-syntax", defaultValue: "Code highlighting: \(key)") }
        switch token {
        case "terminal.background": return String(localized: "studio-describe-background", defaultValue: "The terminal's background")
        case "terminal.foreground": return String(localized: "studio-describe-foreground", defaultValue: "The terminal's text")
        case "terminal.cursor": return String(localized: "studio-describe-cursor", defaultValue: "The terminal's cursor")
        default: return String(localized: "studio-describe-selection", defaultValue: "Selected text in the terminal")
        }
    }
}

private extension String {
    func dropPrefix(_ prefix: String) -> String? { hasPrefix(prefix) ? String(dropFirst(prefix.count)) : nil }
}
