// The theme picker block of Appearance (`theme_picker/`): a filter, then a grid of cards for the themes of each
// appearance, the current pick of each marked. A card is the theme in miniature (its terminal colours and a band
// with its name), so choosing is looking. Picking writes `theme` (dark) or `theme_light`; the window follows
// through `ThemeSync`. A theme that ships several versions has a card per version, on the shelf of its kind: named by the
// family where the shelf holds one version of it, else by the version's own name. Few cards show until "Show all". A theme hidden from the cards (`hidden_themes`: its menu's Hide
// theme) is on no shelf unless it is a pick; "Hidden themes (n)" under the cards lists them, each with Show.

import DesignSystem
import SettingsModel
import SwiftUI
import WordsModel

struct ThemeCards: View {
    let model: SettingsModel
    @State private var filter = ""
    @State private var showAll = false
    @State private var hiddenOpen = false
    @FocusState private var focusedCard: String?
    @Environment(\.tokens) private var tokens

    /// A theme on a card and the value picking it writes (a built-in's name, a file's slug).
    private struct Card: Identifiable {
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

    private static let collapsed = 4
    /// Four to a row, as the GPUI shelves are (`COLUMNS`): the arrow keys walk by it.
    private static let columnCount = 4
    private let columns = Array(repeating: GridItem(.flexible(), spacing: Space._3), count: ThemeCards.columnCount)

    var body: some View {
        VStack(alignment: .leading, spacing: Space._3) {
            if let project = model.projectTheme { projectNotice(project) }
            TextField(filterPlaceholder, text: $filter)
                .textFieldStyle(.plain).textRole(.body)
                .padding(.horizontal, Space._3).frame(height: Metric.control)
                .background(tokens.inputBackground, in: RoundedRectangle(cornerRadius: tokens.metrics.radius))
                .overlay(RoundedRectangle(cornerRadius: tokens.metrics.radius).strokeBorder(tokens.inputBorder, lineWidth: tokens.metrics.borderWidth))
                .frame(maxWidth: 320)
            ForEach(shelves, id: \.self) { appearance in section(appearance) }
            if !filter.isEmpty, shelves.allSatisfy({ cards(appearance: $0).isEmpty }) {
                Text(Words.text("settings-appearance-no-themes-match-the-filter", fallback: "No themes match the filter."))
                    .textRole(.body).foregroundStyle(tokens.textMuted).padding(.vertical, Space._3)
            }
            ImportThemeButton(model: model)
            if !hidden.isEmpty { hiddenList }
        }
        .padding(.vertical, Space._3)
    }

    private var hidden: [String] { model.value("hidden_themes").array?.compactMap(\.string) ?? [] }

    /// One shelf under Dark or Light, two under Match system (`shelves`).
    private var shelves: [ThemeAppearance] {
        switch model.value("theme_mode").string {
        case "dark": [.dark]
        case "light": [.light]
        default: [.dark, .light]
        }
    }

    private func projectNotice(_ project: ProjectThemeMark) -> some View {
        Banner(
            kind: .info,
            text: Words.text("settings-appearance-project-theme-note", ["project": .text(project.name)], fallback: "This terminal shows \(project.name)'s theme. Your own is kept for everything else.")
        ) {
            ActionButton(title: Words.text("settings-appearance-project-settings", fallback: "Project settings"), symbol: nil, frameId: "project-theme-settings") {
                model.show("projects")
                Task { await model.projectsModel?.select(project.root) }
            }
        }
    }

    /// "Filter 60 themes": how many there are to filter, the hidden ones apart.
    private var filterPlaceholder: String {
        let count = shelves.flatMap { cards(appearance: $0, filtered: false) }.count
        return count == 0
            ? Words.text("settings-appearance-filter-themes", fallback: "Filter themes")
            : Words.text("settings-appearance-filter-themes-count", ["count": .whole(count)], fallback: "Filter \(count) theme\(count == 1 ? "" : "s")")
    }

    /// What a card's band says (`theme_picker` `labels`): the family's name when the family has one version of the card's
    /// kind (its shelf then holds only that one), else the version's own name ("Soltari Dim").
    private func label(_ theme: Theme) -> String {
        guard let family = theme.family else { return theme.name }
        let ofKind = ThemeLibrary.bundled.themes.filter { $0.family?.name == family.name && $0.appearance == theme.appearance }
        return ofKind.count == 1 ? family.name : theme.name
    }

    /// The filter box looks in a card's own name and its family's.
    private func matches(_ theme: Theme) -> Bool {
        filter.isEmpty || theme.name.localizedCaseInsensitiveContains(filter) || theme.family?.name.localizedCaseInsensitiveContains(filter) == true
    }

    /// Whether the picker leaves the card out: core's rule over its value, by name where there is no core.
    private func isHidden(_ card: Card) -> Bool {
        model.host.themeHidden(card.hideValue) ?? hidden.contains { $0.localizedCaseInsensitiveCompare(card.theme.name) == .orderedSame }
    }

    private func setHidden(_ value: String, _ on: Bool) {
        if model.host.setThemeHidden(value, hidden: on) { return }
        let rest = hidden.filter { $0.localizedCaseInsensitiveCompare(value) != .orderedSame }
        model.set("hidden_themes", .array((on ? rest + [value] : rest).map(JSONValue.string)))
    }

    /// The cards of one kind, the pick first: the built-in themes then the person's own (`catalog_in`), without the hidden
    /// ones unless one is a pick, and narrowed by the filter box.
    private func cards(appearance: ThemeAppearance, filtered: Bool = true) -> [Card] {
        let picked = model.value(appearance == .dark ? "theme" : "theme_light").string
        let store = ThemeStore.shared
        let yours = store.userThemes.filter { $0.appearance == appearance }.compactMap { summary in
            store.userTheme(summary).map { Card(theme: $0, value: summary.id, path: summary.path, newer: summary.newer) }
        }
        return (ThemeLibrary.bundled.themes.map { Card(theme: $0, value: $0.name) } + yours).filter {
            $0.theme.appearance == appearance && (!filtered || matches($0.theme)) && ($0.isPicked(picked) || !isHidden($0))
        }.sorted { ($0.isPicked(picked) ? 0 : 1) < ($1.isPicked(picked) ? 0 : 1) }
    }

    /// Dark or Light beside a hidden theme's name, from the card it is (none for a name no theme has any more).
    private func kind(ofHidden value: String) -> (name: String, kind: String?) {
        for appearance in ThemeAppearance.allCases {
            let known = ThemeLibrary.bundled.themes.map { Card(theme: $0, value: $0.name) }
                + ThemeStore.shared.userThemes.filter { $0.appearance == appearance }.compactMap { summary in
                    ThemeStore.shared.userTheme(summary).map { Card(theme: $0, value: summary.id, path: summary.path) }
                }
            if let card = known.first(where: { $0.theme.appearance == appearance && ($0.hideValue == value || $0.theme.name.localizedCaseInsensitiveCompare(value) == .orderedSame) }) {
                let kind = appearance == .dark ? Words.text("settings-appearance-dark", fallback: "Dark") : Words.text("settings-appearance-light", fallback: "Light")
                return (label(card.theme), kind)
            }
        }
        return (value, nil)
    }

    /// The disclosure under the cards: a row per hidden theme with Show.
    private var hiddenList: some View {
        VStack(alignment: .leading, spacing: Space._2) {
            LinkButton(
                title: Words.text(
                    "settings-appearance-hidden-themes", ["count": .whole(hidden.count)], fallback: "Hidden themes (\(hidden.count))")
            ) { hiddenOpen.toggle() }
            if hiddenOpen {
                ForEach(hidden, id: \.self) { value in
                    let known = kind(ofHidden: value)
                    HStack {
                        Text(known.name).textRole(.body).foregroundStyle(tokens.textPrimary)
                        if let kind = known.kind { Text(kind).textRole(.caption).foregroundStyle(tokens.textFaint) }
                        Spacer()
                        LinkButton(title: Words.text("settings-appearance-show-theme", fallback: "Show")) { setHidden(value, false) }
                    }
                }
            }
        }
    }

    private func section(_ appearance: ThemeAppearance) -> some View {
        let key = appearance == .dark ? "theme" : "theme_light"
        let picked = model.value(key).string
        let all = cards(appearance: appearance)
        let shown = showAll || !filter.isEmpty ? all : Array(all.prefix(Self.collapsed))
        return VStack(alignment: .leading, spacing: Space._2) {
            HStack {
                Text(
                    appearance == .dark
                        ? Words.text("settings-appearance-in-dark-mode", fallback: "In Dark Mode") : Words.text("settings-appearance-in-light-mode", fallback: "In Light Mode")
                ).textRole(.label).foregroundStyle(tokens.textMuted)
                Spacer()
                if all.count > Self.collapsed, filter.isEmpty {
                    LinkButton(
                        title: showAll
                            ? Words.text("settings-appearance-show-fewer", fallback: "Show fewer")
                            : Words.text("settings-appearance-show-all-themes", ["total": .whole(all.count)], fallback: "Show all \(all.count)")
                    ) { showAll.toggle() }
                }
            }
            LazyVGrid(columns: columns, spacing: Space._3) {
                ForEach(shown) { card in
                    ThemeCard(theme: card.theme, label: label(card.theme), isSelected: card.isPicked(picked), newer: card.newer) { model.set(key, .string(card.value)) }
                        .focused($focusedCard, equals: card.id)
                        .contextMenu {
                            if !card.isPicked(picked) {
                                Button(Words.text("settings-appearance-hide-theme", fallback: "Hide theme")) { setHidden(card.hideValue, true) }
                            }
                            if card.path != nil {
                                Button(Words.text("settings-appearance-move-to-trash", fallback: "Move to Trash") + "…") {
                                    ThemeFiles.trash(card.value, model: model)
                                }
                            }
                        }
                }
            }
            .onKeyPress(keys: [.leftArrow, .rightArrow, .upArrow, .downArrow, .home, .end]) { press in walk(press.key, in: shown) }
        }
    }

    /// The arrow keys walk the cards of a shelf (`neighbor`): by one, by a row, to its first or last.
    private func walk(_ key: KeyEquivalent, in shelf: [Card]) -> KeyPress.Result {
        guard let ix = shelf.firstIndex(where: { $0.id == focusedCard }) else { return .ignored }
        let columns = Self.columnCount
        let next: Int
        switch key {
        case .leftArrow: next = max(ix - 1, 0)
        case .rightArrow: next = min(ix + 1, shelf.count - 1)
        case .upArrow: next = ix >= columns ? ix - columns : ix
        case .downArrow: next = (ix / columns + 1) * columns < shelf.count ? min(ix + columns, shelf.count - 1) : ix
        case .home: next = 0
        case .end: next = shelf.count - 1
        default: return .ignored
        }
        focusedCard = shelf[next].id
        return .handled
    }
}
