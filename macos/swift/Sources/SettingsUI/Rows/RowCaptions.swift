// The sentences GPUI's `describe` puts under a row in place of the table's description, where the words depend on the
// settings: the primary colour in use, the branch colour still to pick, the terminal font's summary, the custom fonts found.

import DesignSystem
import SettingsModel
import SwiftUI
import WordsModel

enum RowCaptions {
    @MainActor static func text(_ entry: SettingsSchema.Entry, _ model: SettingsModel) -> String? {
        switch entry.id {
        case "primary-color": primary(entry, model)
        case "branch-color": branch(model)
        case "terminal-font": terminalFont(model)
        case "custom-fonts": customFonts(model)
        case "open-in": gone(model, key: "open_in", options: model.host.options(for: .openIn), words: Words.text("settings-terminal-not-installed-the-title-row-opens", fallback: "Not installed: the title row opens it in another app"))
        case "default-shell": gone(model, key: "default_shell", options: model.host.options(for: .shells), words: Words.text("settings-terminal-not-found-the-login-shell-runs", fallback: "Not found: the login shell runs instead"))
        default: nil
        }
    }

    /// "The theme's · #7aa2f7. …", "Yours · #hex, in every theme", or "Pick a colour below · the theme's is …".
    @MainActor private static func primary(_ entry: SettingsSchema.Entry, _ model: SettingsModel) -> String {
        let base = ThemeStore.shared.theme.tokens().rgba(named: "primary")?.hex ?? ""
        let usesTheme = model.value("theme_primary").bool ?? true
        let raw = (model.value("primary_color").string ?? "").trimmingCharacters(in: .whitespaces)
        if !usesTheme, Color(hex: raw) != nil {
            return Words.text("settings-appearance-primary-yours", ["color": .text(raw)], fallback: "Yours · \(raw), in every theme")
        }
        if !usesTheme {
            return Words.text("settings-appearance-primary-pick", ["color": .text(base)], fallback: "Pick a colour below · the theme's is \(base)")
        }
        return Words.text(
            "settings-appearance-primary-theme", ["color": .text(base), "description": .text(entry.description)],
            fallback: "The theme's · \(base). \(entry.description)")
    }

    /// While Custom is chosen and no valid colour is picked yet.
    @MainActor private static func branch(_ model: SettingsModel) -> String? {
        guard model.value("branch_color").string == "custom" else { return nil }
        let picked = (model.value("branch_color_custom").string ?? "").trimmingCharacters(in: .whitespaces)
        guard Color(hex: picked) == nil else { return nil }
        let base = ThemeStore.shared.theme.tokens().rgba(named: "git.branch")?.hex ?? ""
        return Words.text("settings-appearance-branch-pick", ["color": .text(base)], fallback: "Pick a colour below · the theme's is \(base)")
    }

    /// A choice written in config.json that names nothing any more (an app that is gone, a shell profile taken out): the row says what
    /// happens instead.
    @MainActor private static func gone(_ model: SettingsModel, key: String, options: [SettingOption], words: String) -> String? {
        let chosen = model.value(key).string ?? ""
        return chosen.isEmpty || options.contains { $0.value == .string(chosen) } ? nil : words
    }

    /// A number set by hand in config.json that the list does not offer, in the list's own words ("75,000 lines", "300 MiB").
    static func handWritten(_ key: String, _ value: JSONValue) -> String? {
        guard let number = value.number else { return nil }
        let grouped = Int(number).formatted(.number.locale(Locale(identifier: "en_US")))
        switch key {
        case "scrollback_lines": return Words.text("settings-terminal-scrollback-lines", ["count": .text(grouped)], fallback: "\(grouped) lines")
        case "image_memory_mib": return Words.text("settings-terminal-image-memory-size", ["mib": .text(String(Int(number)))], fallback: "\(Int(number)) MiB")
        default: return nil
        }
    }

    /// "JetBrains Mono 14 pt · line height 1.30 · ligatures on".
    private static func terminalFont(_ model: SettingsModel) -> String {
        let family = model.value("font_family").string.flatMap { $0.isEmpty ? nil : $0 } ?? "JetBrains Mono" // text: the bundled family's name
        let size = String(format: "%.0f", model.value("font_size").number ?? 14)
        let line = String(format: "%.2f", model.value("line_height").number ?? 1.3)
        let args: [String: WordValue] = ["family": .text(family), "size": .text(size), "line_height": .text(line)]
        return (model.value("font_ligatures").bool ?? true)
            ? Words.text("settings-terminal-font-summary-ligatures-on", args, fallback: "\(family) \(size) pt · line height \(line) · ligatures on")
            : Words.text("settings-terminal-font-summary-ligatures-off", args, fallback: "\(family) \(size) pt · line height \(line) · ligatures off")
    }

    /// "2 families found: Berkeley Mono, Monaspace Neon", or "None yet: …".
    @MainActor private static func customFonts(_ model: SettingsModel) -> String {
        let families = model.backend.fontFolder.families.sorted()
        guard !families.isEmpty else { return Words.text("settings-fonts-none-yet", fallback: "None yet: the fonts folder is empty.") }
        let list = families.joined(separator: ", ")
        return Words.text(
            "settings-fonts-families-found", ["count": .whole(families.count), "families": .text(list)],
            fallback: "\(families.count) \(families.count == 1 ? "family" : "families") found: \(list)")
    }
}
