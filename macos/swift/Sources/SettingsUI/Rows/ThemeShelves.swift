// What the theme picker of Appearance lists (`theme_picker/`), apart from how it is drawn: the shelves the mode asks for,
// each shelf's cards (built-in themes then the person's own, the pick first, the hidden ones left out unless picked,
// narrowed by the filter), what a card's band says, and the hidden themes with their kind. `ThemeCards` draws it; the
// control folder's `theme-cards` reads the same.

import DesignSystem
import SettingsModel
import WordsModel

/// A theme on a card and the value picking it writes (a built-in's name, a file's slug).
struct ThemeCardEntry: Identifiable {
    let theme: Theme
    let value: String
    /// The file `theme` holds for it, when it was imported or saved from the Studio.
    var path: String?
    /// How many entries a file made with a newer Wormhole had that this version skipped.
    var newer: Int?
    /// What `hidden_themes` holds for it (core's `ThemeEntry::value`): a built-in's name, a file's path.
    var hideValue: String { path ?? value }
    var id: String { value }
    func isPicked(_ picked: String?) -> Bool { picked == value || (path != nil && picked == path) }
}

@MainActor
struct ThemeShelves {
    let model: SettingsModel
    let filter: String

    /// How many cards a shelf shows until "Show all".
    nonisolated static let collapsed = 4

    var hidden: [String] { model.value("hidden_themes").array?.compactMap(\.string) ?? [] }

    /// One shelf under Dark or Light, two under Match system (`shelves`).
    var shelves: [ThemeAppearance] {
        switch model.value("theme_mode").string {
        case "dark": [.dark]
        case "light": [.light]
        default: [.dark, .light]
        }
    }

    /// "Filter 60 themes": how many there are to filter, the hidden ones apart.
    var filterPlaceholder: String {
        let count = shelves.flatMap { cards(appearance: $0, filtered: false) }.count
        return count == 0
            ? Words.text("settings-appearance-filter-themes", fallback: "Filter themes")
            : Words.text("settings-appearance-filter-themes-count", ["count": .whole(count)], fallback: "Filter \(count) theme\(count == 1 ? "" : "s")")
    }

    /// The setting a shelf's pick is written to.
    static func key(_ appearance: ThemeAppearance) -> String { appearance == .dark ? "theme" : "theme_light" }

    /// What a card's band says (`theme_picker` `labels`): the family's name when the family has one version of the card's
    /// kind (its shelf then holds only that one), else the version's own name ("Soltari Dim").
    func label(_ theme: Theme) -> String {
        guard let family = theme.family else { return theme.name }
        let ofKind = ThemeLibrary.bundled.themes.filter { $0.family?.name == family.name && $0.appearance == theme.appearance }
        return ofKind.count == 1 ? family.name : theme.name
    }

    /// The filter box looks in a card's own name and its family's.
    func matches(_ theme: Theme) -> Bool {
        filter.isEmpty || theme.name.localizedCaseInsensitiveContains(filter) || theme.family?.name.localizedCaseInsensitiveContains(filter) == true
    }

    /// Whether the picker leaves the card out: core's rule over its value, by name where there is no core.
    func isHidden(_ card: ThemeCardEntry) -> Bool {
        model.host.themeHidden(card.hideValue) ?? hidden.contains { $0.localizedCaseInsensitiveCompare(card.theme.name) == .orderedSame }
    }

    /// The cards of one kind, the pick first: the built-in themes then the person's own (`catalog_in`), without the hidden
    /// ones unless one is a pick, and narrowed by the filter box.
    func cards(appearance: ThemeAppearance, filtered: Bool = true) -> [ThemeCardEntry] {
        let picked = model.value(Self.key(appearance)).string
        let store = ThemeStore.shared
        let yours = store.userThemes.filter { $0.appearance == appearance }.compactMap { summary in
            store.userTheme(summary).map { ThemeCardEntry(theme: $0, value: summary.id, path: summary.path, newer: summary.newer) }
        }
        return (ThemeLibrary.bundled.themes.map { ThemeCardEntry(theme: $0, value: $0.name) } + yours).filter {
            $0.theme.appearance == appearance && (!filtered || matches($0.theme)) && ($0.isPicked(picked) || !isHidden($0))
        }.sorted { ($0.isPicked(picked) ? 0 : 1) < ($1.isPicked(picked) ? 0 : 1) }
    }

    /// Dark or Light beside a hidden theme's name, from the card it is (none for a name no theme has any more).
    func kind(ofHidden value: String) -> (name: String, kind: String?) {
        for appearance in ThemeAppearance.allCases {
            let known = ThemeLibrary.bundled.themes.map { ThemeCardEntry(theme: $0, value: $0.name) }
                + ThemeStore.shared.userThemes.filter { $0.appearance == appearance }.compactMap { summary in
                    ThemeStore.shared.userTheme(summary).map { ThemeCardEntry(theme: $0, value: summary.id, path: summary.path) }
                }
            if let card = known.first(where: { $0.theme.appearance == appearance && ($0.hideValue == value || $0.theme.name.localizedCaseInsensitiveCompare(value) == .orderedSame) }) {
                let kind = appearance == .dark ? Words.text("settings-appearance-dark", fallback: "Dark") : Words.text("settings-appearance-light", fallback: "Light")
                return (label(card.theme), kind)
            }
        }
        return (value, nil)
    }
}
