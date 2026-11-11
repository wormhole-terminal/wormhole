// How a view gets its tokens: `@Environment(\.tokens)`, set once at a window's root by `.themed()`.
// Reading the store inside `themed`'s body is what makes a theme change re-draw the whole tree.

import SwiftUI

extension EnvironmentValues {
    /// The tokens of the theme showing. Outside a `.themed()` tree: the default dark theme.
    @Entry public var tokens: ThemeTokens = ThemeLibrary.bundled.defaultTokens
    /// Reduce Motion as the store has it: macOS's, followed live, unless a check stood in for it (SwiftUI's own
    /// `accessibilityReduceMotion` cannot be set). Outside a `.themed()` tree: off.
    @Entry public var reduceMotion = false
    /// Whether the window is in front as far as the focus rings go: none shows in a window that is not (the window sets it).
    @Entry public var windowFront = true
}

extension ThemeLibrary {
    var defaultTokens: ThemeTokens { (theme(id: defaultDark) ?? themes[0]).tokens() }
}

private struct Themed: ViewModifier {
    let store: ThemeStore

    func body(content: Content) -> some View {
        content
            .environment(\.tokens, store.tokens)
            .environment(\.reduceMotion, store.reduceMotion)
            // Native controls (menus, scrollers, text fields) follow the theme's light or dark, not only macOS's.
            .preferredColorScheme(store.appearance.colorScheme)
    }
}

extension View {
    /// Colours this view and everything under it from `store`'s theme, live.
    @MainActor public func themed(_ store: ThemeStore = .shared) -> some View { modifier(Themed(store: store)) }
}
