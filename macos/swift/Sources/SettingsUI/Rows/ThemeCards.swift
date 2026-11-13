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
    @Environment(\.tokens) private var tokens

    /// A theme on a card and the value picking it writes (a built-in's name, a file's slug).
    private struct Card: Identifiable {
        let theme: Theme
        let value: String
        /// The file `theme` holds for it, when it was imported or saved from the Studio.
        var path: String?
        var id: String { value }
        func isPicked(_ picked: String?) -> Bool { picked == value || (path != nil && picked == path) }
    }

    private static let collapsed = 4
    private let columns = [GridItem(.adaptive(minimum: 140, maximum: 200), spacing: Space._3)]

    var body: some View {
        VStack(alignment: .leading, spacing: Space._3) {
            TextField(Words.text("settings-appearance-filter-themes", fallback: "Filter themes"), text: $filter)
                .textFieldStyle(.plain).textRole(.body)
                .padding(.horizontal, Space._3).frame(height: Metric.control)
                .background(tokens.inputBackground, in: RoundedRectangle(cornerRadius: tokens.metrics.radius))
                .overlay(RoundedRectangle(cornerRadius: tokens.metrics.radius).strokeBorder(tokens.inputBorder, lineWidth: tokens.metrics.borderWidth))
                .frame(maxWidth: 320)
            ForEach(ThemeAppearance.allCases.reversed(), id: \.self) { appearance in section(appearance) }
            ImportThemeButton(model: model)
            if !hidden.isEmpty { hiddenList }
        }
        .padding(.vertical, Space._3)
    }

    private var hidden: [String] { model.value("hidden_themes").array?.compactMap(\.string) ?? [] }

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

    private func isHidden(_ name: String) -> Bool { hidden.contains { $0.localizedCaseInsensitiveCompare(name) == .orderedSame } }

    private func setHidden(_ name: String, _ on: Bool) {
        let rest = hidden.filter { $0.localizedCaseInsensitiveCompare(name) != .orderedSame }
        model.set("hidden_themes", .array((on ? rest + [name] : rest).map(JSONValue.string)))
    }

    /// The disclosure under the cards: a row per hidden theme with Show.
    private var hiddenList: some View {
        VStack(alignment: .leading, spacing: Space._2) {
            LinkButton(
                title: Words.text(
                    "settings-appearance-hidden-themes", ["count": .whole(hidden.count)], fallback: "Hidden themes (\(hidden.count))")
            ) { hiddenOpen.toggle() }
            if hiddenOpen {
                ForEach(hidden, id: \.self) { name in
                    HStack {
                        Text(name).textRole(.body).foregroundStyle(tokens.textPrimary)
                        Spacer()
                        LinkButton(title: Words.text("settings-appearance-show-theme", fallback: "Show")) { setHidden(name, false) }
                    }
                }
            }
        }
    }

    private func section(_ appearance: ThemeAppearance) -> some View {
        let key = appearance == .dark ? "theme" : "theme_light"
        let picked = model.value(key).string
        // The current pick leads, so it is in view before "Show all".
        // The person's own themes follow the built-in ones (`catalog_in`).
        let store = ThemeStore.shared
        let yours = store.userThemes.filter { $0.appearance == appearance }.compactMap { summary in
            store.userTheme(summary).map { Card(theme: $0, value: summary.id, path: summary.path) }
        }
        let all = (ThemeLibrary.bundled.themes.map { Card(theme: $0, value: $0.name) } + yours).filter {
            $0.theme.appearance == appearance && matches($0.theme)
                && ($0.isPicked(picked) || !isHidden($0.theme.name))
        }.sorted { ($0.isPicked(picked) ? 0 : 1) < ($1.isPicked(picked) ? 0 : 1) }
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
                    ThemeCard(theme: card.theme, label: label(card.theme), isSelected: card.isPicked(picked)) { model.set(key, .string(card.value)) }
                        .contextMenu {
                            if !card.isPicked(picked) {
                                Button(Words.text("settings-appearance-hide-theme", fallback: "Hide theme")) { setHidden(card.theme.name, true) }
                            }
                            if card.path != nil {
                                Button(Words.text("settings-appearance-move-to-trash", fallback: "Move to Trash") + "…") {
                                    ThemeFiles.trash(card.value, model: model)
                                }
                            }
                        }
                }
            }
        }
    }
}
