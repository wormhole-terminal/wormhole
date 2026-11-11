@testable import DesignSystem

/// A theme's tokens with some colours replaced, for a contrast test that needs a weak one.
enum MockTokens {
    static func with(_ tokens: ThemeTokens, _ replacing: [String: RGBA]) -> ThemeTokens? {
        var ui = tokens.ui
        for (name, color) in replacing { ui[name] = color }
        return ThemeTokens(terminal: tokens.terminal, metrics: tokens.metrics, ui: ui, syntax: tokens.syntax, translucent: nil)
    }
}
