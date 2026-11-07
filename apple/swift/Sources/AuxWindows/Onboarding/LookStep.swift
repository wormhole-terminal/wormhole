// "Make it yours": theme cards, the primary colour, the terminal font and ligatures, and a sample line of
// terminal in the colours chosen (`look_step.rs`). Everything changes live. The theme goes through the store the
// whole app draws from; the rest are the backend's settings (`OnboardingPreferences`).

import AuxModel
import DesignSystem
import SwiftUI

struct LookStep: View {
    let preferences: OnboardingPreferences
    var store = ThemeStore.shared
    @Environment(\.tokens) private var tokens

    /// The themes the step offers, if the build has them (more are in Settings): `onboarding::LOOK_THEMES`.
    static let offered = ["Wormhole Dark", "Tokyo Night", "Catppuccin Mocha", "Nord", "Rose Pine", "Wormhole Light"]
    /// The terminal colours offered as primaries beside the theme's own: ANSI green, magenta, red, yellow, cyan.
    static let primaryAnsi = [2, 5, 1, 3, 6]

    var body: some View {
        let themes = Self.offered.compactMap { name in store.library.themes.first { $0.name == name } }
        VStack(alignment: .leading, spacing: Space._3) {
            LazyVGrid(columns: Array(repeating: GridItem(.flexible(), spacing: Space._2), count: 3), spacing: Space._2) {
                ForEach(themes) { theme in
                    ThemeCard(theme: theme, current: theme.id == store.theme.id) { store.setTheme(theme.id) }
                }
            }
            row(AuxText.primaryColour) { swatches }
            row(AuxText.terminalFont) { font }
            row(AuxText.ligatures) {
                Toggle(AuxText.ligatures, isOn: Bindable(preferences).ligatures).toggleStyle(.switch).tint(tokens.primary).labelsHidden()
            }
            sample.padding(.top, Space._1)
        }
        .padding(.horizontal, Space._6).padding(.bottom, Space._4)
    }

    private func row<Control: View>(_ label: String, @ViewBuilder control: () -> Control) -> some View {
        HStack(spacing: Space._4) {
            Text(label).font(TextRole.body.font)
            Spacer(minLength: 0)
            control()
        }
        .frame(minHeight: Metric.control)
        .accessibilityElement(children: .contain)
    }

    private var swatches: some View {
        let theme = store.tokens
        let colours = [("", theme.rgba(named: "primary"))] + Self.primaryAnsi.map { index in
            (theme.terminal.ansi[index].hex, Optional(theme.terminal.ansi[index]))
        }
        return HStack(spacing: Space._1) {
            ForEach(Array(colours.enumerated()), id: \.offset) { _, entry in
                let chosen = entry.0.isEmpty ? preferences.primary == nil : preferences.primary == entry.0
                Button { preferences.primary = entry.0.isEmpty ? nil : entry.0 } label: {
                    Circle().fill(entry.1?.color ?? tokens.primary).frame(width: Metric.glyph + Space._2, height: Metric.glyph + Space._2)
                        .overlay { Circle().strokeBorder(tokens.textPrimary, lineWidth: chosen ? Metric.ring : 0).padding(-Space._1) }
                }
                .buttonStyle(.plain)
                .accessibilityLabel(entry.0.isEmpty ? "\(AuxText.primaryColour): \(AuxText.themeLabel)" : entry.0)
                .accessibilityAddTraits(chosen ? .isSelected : [])
            }
        }
        .padding(Space._1)
    }

    private var font: some View {
        HStack(spacing: Space._2) {
            Text(preferences.fontFamily).font(TextRole.body.font).lineLimit(1)
                .padding(.horizontal, Space._2).frame(height: Metric.control)
                .background(tokens.inputBackground, in: RoundedRectangle(cornerRadius: tokens.metrics.radius))
                .overlay { RoundedRectangle(cornerRadius: tokens.metrics.radius).strokeBorder(tokens.border, lineWidth: tokens.metrics.borderWidth) }
            Stepper(AuxText.fontSize, value: Bindable(preferences).fontSize, in: 8...32)
                .labelsHidden()
            Text(preferences.fontSize.formatted(.number.precision(.fractionLength(0)))).font(TextRole.body.font.monospacedDigit())
        }
    }

    private var sample: some View {
        let terminal = store.tokens.terminal
        let font = Font.custom(preferences.fontFamily, size: TextRole.caption.size)
        return VStack(alignment: .leading, spacing: 0) {
            Text("~/Projects/app on main").foregroundStyle(tokens.textMuted)
            Text("$ git status --short").foregroundStyle(terminal.foreground.color)
            Text(" M src/main.rs").foregroundStyle(terminal.ansi[3].color)
            Text("?? docs/notes.md").foregroundStyle(terminal.ansi[1].color)
        }
        .font(font)
        .padding(.horizontal, Space._3).padding(.vertical, Space._2)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(terminal.background.color, in: RoundedRectangle(cornerRadius: tokens.metrics.radius))
        .overlay { RoundedRectangle(cornerRadius: tokens.metrics.radius).strokeBorder(tokens.border, lineWidth: tokens.metrics.borderWidth) }
        .accessibilityHidden(true)
    }
}
