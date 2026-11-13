// Every colour the UI draws, resolved by the Rust token rules for one theme in one contrast setting.
// The names are the Rust tokens' (`sidebar.row.selected` is `sidebarRowSelected`, `text.on_primary` is
// `textOnPrimary`; the generated files `Tokens*.swift` hold one accessor each). Nothing here derives a
// colour: derivation lives in `wormhole-core` and arrives through `Resources/themes.json`.

import SwiftUI

public struct ThemeTokens: Sendable, Equatable {
    public let terminal: TerminalPalette
    public let metrics: ThemeMetrics
    let ui: [String: RGBA]
    let syntax: [String: SyntaxStyle]
    /// The text core judged for a window at the settings' default look, when the variant carries it.
    let translucent: TranslucentText?

    /// What a missing token shows as, magenta like the GPUI app's, so a typo is easy to spot.
    public static let missing = RGBA(1, 0, 1)

    /// A token by its Rust name (`settings.group`); `nil` when the theme has none.
    public func rgba(named name: String) -> RGBA? { ui[name] }

    /// A token by its Rust name; magenta for a name that doesn't exist.
    public func color(named name: String) -> Color { (ui[name] ?? Self.missing).color }

    /// The colour of a syntax key (`keyword`, `string`). A key the theme does not style takes its first segment's
    /// (`tag.doctype` is `tag`, as the GPUI app's syntax theme resolves it), and an unknown one falls back to `variable`, as in Rust.
    public func syntaxStyle(_ key: String) -> SyntaxStyle? {
        syntax[key] ?? key.split(separator: ".").first.flatMap { syntax[String($0)] } ?? syntax["variable"]
    }

    /// Every UI token name, sorted: what the Studio and tests walk.
    public var names: [String] { ui.keys.sorted() }

    init(
        terminal: TerminalPalette, metrics: ThemeMetrics, ui: [String: RGBA], syntax: [String: SyntaxStyle],
        translucent: TranslucentText? = nil
    ) {
        (self.terminal, self.metrics, self.ui, self.syntax) = (terminal, metrics, ui, syntax)
        self.translucent = translucent
    }
}
