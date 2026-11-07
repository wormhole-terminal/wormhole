// A bundled theme: its identity and its two sets of tokens, as shown normally and under Increase Contrast.

public struct Theme: Sendable, Identifiable, Equatable {
    /// The slug that selects it (`wormhole.dark`, `nord`), as `settings.theme` takes it.
    public let id: String
    public let name: String
    /// A theme that ships several versions: ("Soltari", "Light").
    public let family: (name: String, version: String)?
    public let appearance: ThemeAppearance
    let normal: ThemeTokens
    let highContrast: ThemeTokens

    /// The tokens for the person's Increase Contrast setting.
    public func tokens(increaseContrast: Bool = false) -> ThemeTokens { increaseContrast ? highContrast : normal }

    public static func == (lhs: Theme, rhs: Theme) -> Bool { lhs.id == rhs.id }
}
