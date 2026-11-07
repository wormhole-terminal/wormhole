// Carries the theme settings into the window's `ThemeStore`, so a pick on the Appearance page re-colours the
// whole window (and every other open one) at once. The picks (`theme`, `theme_light`) are set first and the mode
// last: setting a pick of the other appearance would otherwise move the mode, and the mode here is the stored one.

import DesignSystem
import SettingsModel

@MainActor
public enum ThemeSync {
    public static func apply(_ values: [String: JSONValue], to store: ThemeStore = .shared) {
        for key in ["theme", "theme_light"] {
            if let name = values[key]?.string, let theme = store.library.themes.first(where: { $0.name == name }) {
                store.setTheme(theme.id)
            }
        }
        if let raw = values["theme_mode"]?.string, let mode = ThemeMode(rawValue: raw) { store.mode = mode }
    }
}
