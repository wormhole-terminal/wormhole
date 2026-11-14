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

    private typealias Card = ThemeCardEntry

    /// What the shelves list, as `theme-cards` reads it too.
    private var listing: ThemeShelves { ThemeShelves(model: model, filter: filter) }

    private static let collapsed = ThemeShelves.collapsed
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

    private var hidden: [String] { listing.hidden }
    private var shelves: [ThemeAppearance] { listing.shelves }

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

    private var filterPlaceholder: String { listing.filterPlaceholder }
    private func label(_ theme: Theme) -> String { listing.label(theme) }

    private func setHidden(_ value: String, _ on: Bool) {
        if model.host.setThemeHidden(value, hidden: on) { return }
        let rest = hidden.filter { $0.localizedCaseInsensitiveCompare(value) != .orderedSame }
        model.set("hidden_themes", .array((on ? rest + [value] : rest).map(JSONValue.string)))
    }

    private func cards(appearance: ThemeAppearance, filtered: Bool = true) -> [Card] { listing.cards(appearance: appearance, filtered: filtered) }
    private func kind(ofHidden value: String) -> (name: String, kind: String?) { listing.kind(ofHidden: value) }

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
