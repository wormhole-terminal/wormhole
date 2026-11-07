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

    /// What a missing token shows as, magenta like the GPUI app's, so a typo is easy to spot.
    public static let missing = RGBA(1, 0, 1)

    /// A token by its Rust name (`settings.group`); `nil` when the theme has none.
    public func rgba(named name: String) -> RGBA? { ui[name] }

    /// A token by its Rust name; magenta for a name that doesn't exist.
    public func color(named name: String) -> Color { (ui[name] ?? Self.missing).color }

    /// The colour of a syntax key (`keyword`, `string`); an unknown key falls back to `variable`, as in Rust.
    public func syntaxStyle(_ key: String) -> SyntaxStyle? { syntax[key] ?? syntax["variable"] }

    /// Every UI token name, sorted: what the Studio and tests walk.
    public var names: [String] { ui.keys.sorted() }

    init(terminal: TerminalPalette, metrics: ThemeMetrics, ui: [String: RGBA], syntax: [String: SyntaxStyle]) {
        (self.terminal, self.metrics, self.ui, self.syntax) = (terminal, metrics, ui, syntax)
    }
}
