// Any colour a theme has by its token name: a UI token, a terminal colour (`terminal.background`), a palette
// slot (`terminal.ansi.4`) or a syntax colour (`syntax.keyword`). `ThemeTokens.rgba(named:)` is the UI tokens only.

import DesignSystem

extension ThemeTokens {
    public func colorValue(of token: String) -> RGBA? {
        switch TokenPath(token) {
        case .ansi(let slot): return terminal.ansi[slot]
        case .syntax(let key): return syntaxStyle(key)?.color
        case .terminal(let key):
            switch key {
            case "background": return terminal.background
            case "foreground": return terminal.foreground
            case "cursor": return terminal.cursor
            case "cursor_text": return terminal.cursorText
            case "selection_background": return terminal.selectionBackground
            case "selection_foreground": return terminal.selectionForeground
            default: return nil
            }
        case .metric: return nil
        case .ui(let name): return rgba(named: name)
        }
    }
}
