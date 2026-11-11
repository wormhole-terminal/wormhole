// Carries the theme settings into the window's `ThemeStore`, so a pick on the Appearance page re-colours the
// whole window (and every other open one) at once. The picks (`theme`, `theme_light`) are set first and the mode
// last: setting a pick of the other appearance would otherwise move the mode, and the mode here is the stored one.
// The accent picks go with them (`Settings::primary` and `branch_tint`: an unusable colour is the theme's own, as
// there), and so does Borders.

import DesignSystem
import Foundation
import SettingsModel

@MainActor
public enum ThemeSync {
    public static func apply(_ values: [String: JSONValue], to store: ThemeStore = .shared) {
        applyPicks(values, to: store)
        if let raw = values["theme_mode"]?.string, let mode = ThemeMode(rawValue: raw) { store.mode = mode }
        applyAccents(values, to: store)
        store.showsBorders = values["borders"]?.bool ?? true
        store.textScale = SettingsMetric.textScale(values["settings_text_size"]?.string)
        var secondary = SecondaryLook()
        secondary.settings = values["settings_translucent"]?.bool ?? true
        secondary.studio = values["studio_translucent"]?.bool ?? true
        secondary.hosts = values["hosts_translucent"]?.bool ?? true
        secondary.opacity = values["opacity"]?.number.map { min(max($0, 0.3), 1) } ?? 0.8
        secondary.blur = values["blur"]?.bool ?? true
        if secondary != store.secondary { store.secondary = secondary }
        store.interfaceFont = InterfaceFont(
            family: values["ui_font_family"]?.string ?? "", size: values["ui_font_size"]?.number ?? InterfaceFont.defaultSize)
    }

    /// The picks as `config.json` holds them: a bundled theme by its name or slug, anything else (a theme of the person's
    /// own, by name, slug or path) as written, for core's resolver to find (`theme_resolve::find`). A pick that does not
    /// resolve is reported (`ThemeStore.problem`) and the default shows until it is fixed, as in the GPUI app.
    static func applyPicks(_ values: [String: JSONValue], to store: ThemeStore) {
        var next = store.settings
        for (key, dark) in [("theme", true), ("theme_light", false)] {
            guard let raw = values[key]?.string?.trimmingCharacters(in: .whitespaces), !raw.isEmpty else { continue }
            let pick = store.library.themes.first { $0.id == raw || $0.name == raw }?.id ?? raw
            if dark { next.darkTheme = pick } else { next.lightTheme = pick }
        }
        if next != store.settings { store.settings = next }
    }

    /// `primary_color` while `theme_primary` is off (a config from before the switch: while a colour is set), and
    /// `branch_color`'s pick of the theme's, the primary or its own colour.
    static func applyAccents(_ values: [String: JSONValue], to store: ThemeStore) {
        let picked = values["primary_color"]?.string?.trimmingCharacters(in: .whitespaces) ?? ""
        let themes = values["theme_primary"]?.bool ?? picked.isEmpty
        var next = store.settings
        next.primary = themes || !isHex(picked) ? "" : picked
        let custom = values["branch_color_custom"]?.string?.trimmingCharacters(in: .whitespaces) ?? ""
        switch values["branch_color"]?.string {
        case "primary": next.branch = .primary
        case "custom" where isHex(custom): next.branch = .custom(custom)
        default: next.branch = .theme
        }
        if next != store.settings { store.settings = next }
    }

    /// `#rgb`, `#rrggbb` or `#rrggbbaa`: what core's colour reader takes.
    private static func isHex(_ text: String) -> Bool {
        text.hasPrefix("#") && [4, 5, 7, 9].contains(text.count) && text.dropFirst().allSatisfy(\.isHexDigit)
    }
}
