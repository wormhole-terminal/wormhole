// Light or dark. A theme is one or the other by itself (its background decides); the person picks
// a theme for each, and the mode says which of the two shows (`ThemeMode`).

import SwiftUI

public enum ThemeAppearance: String, Sendable, Codable, CaseIterable {
    case light, dark

    public var colorScheme: ColorScheme { self == .dark ? .dark : .light }
}

/// Settings → Appearance → Mode: a fixed appearance, or whichever macOS is in (the app's default).
public enum ThemeMode: String, Sendable, Codable, CaseIterable {
    case system, light, dark

    /// The appearance this mode shows, given the one macOS is in.
    public func appearance(system: ThemeAppearance) -> ThemeAppearance {
        switch self {
        case .system: system
        case .light: .light
        case .dark: .dark
        }
    }
}
