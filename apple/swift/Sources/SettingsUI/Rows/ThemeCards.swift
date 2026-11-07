// The theme picker block of Appearance (`theme_picker/`): a filter, then a grid of cards for the themes of each
// appearance, the current pick of each marked. A card is the theme in miniature (its terminal colours and a band
// with its name), so choosing is looking. Picking writes `theme` (dark) or `theme_light`; the window follows
// through `ThemeSync`. Few cards show until "Show all".

import DesignSystem
import SettingsModel
import SwiftUI
import WordsModel

struct ThemeCards: View {
    let model: SettingsModel
    @State private var filter = ""
    @State private var showAll = false
    @Environment(\.tokens) private var tokens

    private static let collapsed = 4
    private let columns = [GridItem(.adaptive(minimum: 140, maximum: 200), spacing: Space._3)]

    var body: some View {
        VStack(alignment: .leading, spacing: Space._3) {
            TextField(Words.text("settings-appearance-filter-themes", fallback: "Filter themes"), text: $filter)
                .textFieldStyle(.plain).font(.system(size: TextRole.body.size))
                .padding(.horizontal, Space._3).frame(height: Metric.control)
                .background(tokens.inputBackground, in: RoundedRectangle(cornerRadius: tokens.metrics.radius))
                .overlay(RoundedRectangle(cornerRadius: tokens.metrics.radius).strokeBorder(tokens.inputBorder, lineWidth: tokens.metrics.borderWidth))
                .frame(maxWidth: 320)
            ForEach(ThemeAppearance.allCases.reversed(), id: \.self) { appearance in section(appearance) }
        }
        .padding(.vertical, Space._3)
    }

    private func section(_ appearance: ThemeAppearance) -> some View {
        let key = appearance == .dark ? "theme" : "theme_light"
        let picked = model.value(key).string
        // The current pick leads, so it is in view before "Show all".
        let all = ThemeLibrary.bundled.themes.filter {
            $0.appearance == appearance && (filter.isEmpty || $0.name.localizedCaseInsensitiveContains(filter))
        }.sorted { ($0.name == picked ? 0 : 1) < ($1.name == picked ? 0 : 1) }
        let shown = showAll || !filter.isEmpty ? all : Array(all.prefix(Self.collapsed))
        return VStack(alignment: .leading, spacing: Space._2) {
            HStack {
                Text(appearance == .dark ? "In Dark Mode" : "In Light Mode").font(TextRole.label.font).foregroundStyle(tokens.textMuted)
                Spacer()
                if all.count > Self.collapsed, filter.isEmpty {
                    LinkButton(title: showAll ? "Show fewer" : "Show all \(all.count)") { showAll.toggle() }
                }
            }
            LazyVGrid(columns: columns, spacing: Space._3) {
                ForEach(shown) { theme in
                    ThemeCard(theme: theme, isSelected: model.value(key).string == theme.name) { model.set(key, .string(theme.name)) }
                }
            }
        }
    }
}
