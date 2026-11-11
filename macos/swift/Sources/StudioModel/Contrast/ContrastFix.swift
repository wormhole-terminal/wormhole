// A way out of a pair that falls short, written as an expression the person applies or not: the Studio never
// changes a value by itself. The fix raises the text, `readable(<what it is now>, <backgrounds>, <min>)`, over
// every background it falls short on (themes.md: "readable moves X only as far as it needs to"). Core's fix
// also follows tokens that merely name a failing one and spots cycles; here the resolver reports a cycle.

import DesignSystem

public struct ContrastFix: Sendable, Equatable {
    public let token: String
    public let expression: String
    public let label: String
}

public struct ContrastIssue: Sendable, Equatable {
    public let check: ContrastCheck
    public let fix: ContrastFix?
}

extension ContrastCheck {
    /// The failing pairs, each with the fix for its text. The terminal palette's own pair has none: it is the
    /// palette, not the interface. `source` is what a token is made of now (typed, or its rule).
    public static func issues(in tokens: ThemeTokens, source: (String) -> String?) -> [ContrastIssue] {
        let failing = all(in: tokens).filter { !$0.passes }
        return failing.map { check in
            guard !check.fg.hasPrefix("terminal.") else { return ContrastIssue(check: check, fix: nil) }
            let group = failing.filter { $0.fg == check.fg }
            return ContrastIssue(check: check, fix: fix(for: check.fg, group: group, tokens: tokens, source: source))
        }
    }

    private static func fix(for token: String, group: [ContrastCheck], tokens: ThemeTokens, source: (String) -> String?) -> ContrastFix? {
        var terms: [String] = []
        for check in group {
            let translucent = (tokens.colorValue(of: check.bg)?.alpha ?? 1) < 1
            let term = translucent && check.pair.underlay != "surface.window" ? "over(\(check.bg), \(check.pair.underlay))" : check.bg // text: a theme expression, not words
            if !terms.contains(term) { terms.append(term) }
        }
        let floor = group.map(\.min).max() ?? 0
        let current = source(token) ?? tokens.colorValue(of: token)?.hex ?? token
        let literal = current.hasPrefix("#")
        let ratio = floor == floor.rounded() ? String(Int(floor)) : String(floor)
        let label: String
        if literal, let text = tokens.colorValue(of: token), let bg = tokens.colorValue(of: group[0].bg) {
            label = luminance(text) >= luminance(bg) ? StudioText.lighten(ratio) : StudioText.darken(ratio)
        } else {
            label = StudioText.useReadable(current.count > 24 ? String(current.prefix(23)) + "…" : current, ratio)
        }
        return ContrastFix(token: token, expression: "readable(\(current), \(terms.joined(separator: ", ")), \(ratio))", label: label)  // text: a theme expression, not words
    }

}
