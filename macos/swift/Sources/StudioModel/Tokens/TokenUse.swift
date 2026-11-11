// A component of the design and the tokens it draws with (`theme/tokens/uses.rs` `TokenUse`): what the Studio
// outlines in the gallery while its token is selected, what "Used by" names, and what the area filter keeps.
// `terminal.*` and `syntax.*` stand for every terminal or syntax colour.

public struct TokenUse: Sendable, Equatable, Identifiable {
    public let id: String
    public let label: String
    public let area: StudioArea
    public let tokens: [String]

    public init(id: String, label: String, area: StudioArea, tokens: [String]) {
        (self.id, self.label, self.area, self.tokens) = (id, label, area, tokens)
    }

    /// Whether the component draws with `token` (a terminal or syntax colour matches its family's entry).
    public func draws(with token: String) -> Bool {
        tokens.contains(token)
            || (token.hasPrefix("terminal.") && tokens.contains("terminal.*"))
            || (token.hasPrefix("syntax.") && tokens.contains("syntax.*"))
    }
}
