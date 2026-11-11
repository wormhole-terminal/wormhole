// `ThemePicking` over the settings host: the person's picks are written to `config.json` by core's rule and read back
// through the settings follower into the theme store (`ThemeSync`), so every window follows and nothing is held twice.

#if WITH_UNIFFI
    import DesignSystem
    import SettingsBridge

    @MainActor
    final class ThemePicks: ThemePicking {
        private let host: RustSettingsHost

        init(host: RustSettingsHost) { self.host = host }

        func pick(_ value: String, dark: Bool) { host.pickTheme(value, dark: dark) }
        func pickAfterSave(_ value: String, dark: Bool, path: String) -> Bool { host.pickThemeAfterSave(value, dark: dark, path: path) }
        func renamePicks(_ renames: [(old: String, new: String)]) { host.renameThemePicks(renames) }
        func followSystem(light: String, dark: String) { host.followSystemWith(light: light, dark: dark) }
        func selects(_ setting: String, path: String) -> Bool { host.themeSelects(setting, path: path) }
    }
#endif
