// What a client that resolves themes itself (the Theme Studio, on a draft file) needs from the resolver's
// answer: the whole theme to show, a metric by name, and the syntax keys the theme styles.

import Foundation

extension Theme {
    /// A theme from the resolver's JSON (as `resolve_theme` writes it); throws when it is not that shape.
    public init(resolvedJSON data: Data) throws { self = try ResolvedTheme(json: data).theme }
}

extension ThemeTokens {
    /// A `metrics.*` token's value (`metrics.radius`), or `nil` for a name that is not a metric.
    public func metric(named name: String) -> Double? {
        switch name {
        case "metrics.radius_sm": metrics.radiusSm
        case "metrics.radius_chip": metrics.radiusChip
        case "metrics.radius": metrics.radius
        case "metrics.radius_lg": metrics.radiusLg
        case "metrics.border_width": metrics.borderWidth
        case "metrics.divider_width": metrics.dividerWidth
        case "metrics.focus_ring_width": metrics.focusRingWidth
        default: nil
        }
    }

    /// The syntax keys the theme styles, sorted (`keyword`, `string.escape`).
    public var syntaxKeys: [String] { syntax.keys.sorted() }
}
