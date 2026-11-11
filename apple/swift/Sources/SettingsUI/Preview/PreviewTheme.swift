// The tokens a scene draws with: the theme the settings pick for the appearance in force, not the settings
// window's own (they differ for a moment while a pick is applied, and always in the theme cards).

import DesignSystem
import SettingsModel

@MainActor
enum PreviewTheme {
    static func tokens(_ model: SettingsModel) -> ThemeTokens {
        let store = ThemeStore.shared
        let mode = ThemeMode(rawValue: model.value("theme_mode").string ?? "") ?? .system
        let dark = mode.appearance(system: store.systemAppearance) == .dark
        let name = model.value(dark ? "theme" : "theme_light").string
        let theme = store.library.themes.first { $0.name == name } ?? store.theme
        return theme.tokens(increaseContrast: store.increaseContrast)
    }
}
