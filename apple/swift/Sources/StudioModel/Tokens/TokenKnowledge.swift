// What the Studio knows about tokens that is core's: each one's derivation rule and what follows it. A protocol,
// so the FFI's `default_rule` can replace the generated table (`CoreRules`, written from the rule file by
// `apple/scripts/gen-token-rules.py`) without the Studio changing.

import DesignSystem

public protocol TokenKnowledge: Sendable {
    /// The rule that derives `token` when the theme doesn't set it, in themes.md's expression syntax.
    func rule(for token: String, appearance: ThemeAppearance) -> String?
    /// The components and what they draw with (the gallery's specimens): who uses a token, and in which area.
    var uses: [TokenUse] { get }
}

extension TokenKnowledge {
    /// The syntax colours among `keys` that have a rule of their own; the rest follow one of them, so one value
    /// moves a whole family (`keyword.control` follows `keyword`).
    public func syntaxRoots(among keys: [String]) -> [String] {
        keys.filter { rule(for: "syntax.\($0)", appearance: .dark).map { !$0.hasPrefix("syntax.") } ?? false }
    }

    /// The tokens whose rule names `token`: what an edit of it moves ("1 token follows it").
    public func followers(of token: String, among names: [String], appearance: ThemeAppearance) -> [String] {
        names.filter { name in
            guard name != token, let rule = rule(for: name, appearance: appearance) else { return false }
            return Self.names(in: rule).contains(token)
        }
    }

    /// The token names an expression refers to (`mix(primary, surface.window, 30%)` → primary, surface.window).
    public static func names(in rule: String) -> [String] {
        rule.split(whereSeparator: { !($0.isLetter || $0.isNumber || $0 == "." || $0 == "_") })
            .map(String.init).filter { $0.first?.isLetter == true }
    }
}

/// `CoreRules`, the generated table.
public struct CoreKnowledge: TokenKnowledge {
    public init() {}

    public var uses: [TokenUse] { [] }

    public func rule(for token: String, appearance: ThemeAppearance) -> String? {
        guard let rule = CoreRules.table[token] else { return nil }
        return appearance == .dark ? rule.dark : rule.light
    }
}
