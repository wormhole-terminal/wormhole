// What a token is when the catalogue has no line for it: a palette slot, a syntax colour, the terminal's own
// colours (`describe` in `studio/token_editor.rs`).

import Foundation
import WordsModel

enum StudioDescriptions {
    static func describe(_ token: String) -> String {
        if let slot = token.dropPrefix("terminal.ansi.") { return Words.text("studio-describe-ansi", fallback: "Terminal palette color \(slot)") }
        if let key = token.dropPrefix("syntax.") { return Words.text("studio-describe-syntax", fallback: "Code highlighting: \(key)") }
        switch token {
        case "terminal.background": return Words.text("studio-describe-background", fallback: "The terminal's background")
        case "terminal.foreground": return Words.text("studio-describe-foreground", fallback: "The terminal's text")
        case "terminal.cursor": return Words.text("studio-describe-cursor", fallback: "The terminal's cursor")
        default: return Words.text("studio-describe-selection", fallback: "Selected text in the terminal")
        }
    }
}

private extension String {
    func dropPrefix(_ prefix: String) -> String? { hasPrefix(prefix) ? String(dropFirst(prefix.count)) : nil }
}
