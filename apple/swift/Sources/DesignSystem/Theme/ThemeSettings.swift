// What the person chose about looks, as one value: which theme for dark and for light, the mode, the primary
// and branch colour picks, the window's opacity and blur. Saved as JSON in the Swift app's own defaults
// suite (the Swift app's bundle id plus `.theme`), which is not the GPUI app's `config.json` and not its preferences.

import Foundation

public struct ThemeSettings: Sendable, Equatable, Codable {
    /// A bundled theme's slug, a user theme's name or slug, or a path: what `settings.theme` holds.
    public var darkTheme: String
    public var lightTheme: String
    public var mode: ThemeMode = .system
    /// `#rrggbb` replaces every theme's primary; empty keeps the theme's own ("Use the theme's primary colour").
    public var primary: String = ""
    public var branch: BranchChoice = .theme
    public var look: WindowLook

    public enum BranchChoice: Sendable, Equatable, Codable {
        case theme, primary
        case custom(String)

        /// The resolver's spelling: empty, `primary`, or `#rrggbb`.
        var spelling: String {
            switch self {
            case .theme: ""
            case .primary: "primary"
            case .custom(let hex): hex
            }
        }
    }

    public init(library: ThemeLibrary) {
        darkTheme = library.defaultDark
        lightTheme = library.defaultLight
        look = library.defaultLook
    }
}
