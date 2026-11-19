// What GPUI hangs on a row besides its control (`Setting::below`, `.chip`, `.describe`): the warnings and notes under the
// Appearance rows, colour swatches under the two colour rows, the Beta and Time Sensitive tags. The conditions are core's
// (`blurIsUnsafe`) or the system's (Reduce Transparency, Increase Contrast); the words are `settings-appearance.ftl`'s.

import DesignSystem
import SettingsModel
import SwiftUI
import WordsModel

enum RowExtras {
    /// A tag after the row's label.
    @MainActor static func chip(_ entry: SettingsSchema.Entry, _ model: SettingsModel) -> String? {
        switch entry.id {
        case "liquid-glass": Words.text("settings-appearance-liquid-glass-beta", fallback: "Beta")
        case "notify-needs-you":
            model.notifyStatus.timeSensitive ? Words.text("settings-notifications-time-sensitive", fallback: "Time Sensitive") : nil
        default: nil
        }
    }

    /// The view under the row, for the rows that have one.
    @MainActor static func below(_ entry: SettingsSchema.Entry, _ model: SettingsModel) -> AnyView? {
        switch entry.id {
        case "opacity": AnyView(OpaqueNote())
        case "blur": AnyView(BlurWarning(model: model, entry: entry))
        case "liquid-glass": AnyView(GlassNote(model: model))
        case "primary-color": AnyView(PrimarySwatches(model: model, entry: entry))
        case "branch-color": AnyView(BranchSwatches(model: model, entry: entry))
        default: nil
        }
    }

    /// The project's folder sets this row: its swatches and buttons are held back (`project_sets`).
    @MainActor static func projectSets(_ entry: SettingsSchema.Entry, _ model: SettingsModel) -> Bool {
        model.host.panelRecords().project?.entries.contains(entry.id) ?? false
    }
}

/// While Reduce Transparency or Increase Contrast makes every window opaque the slider is off, and this says why.
private struct OpaqueNote: View {
    @Environment(\.reduceTransparency) private var reduceTransparency
    @Environment(\.increaseContrast) private var increaseContrast

    var body: some View {
        if reduceTransparency || increaseContrast {
            Banner(
                kind: .info,
                text: reduceTransparency
                    ? Words.text("settings-appearance-opaque-reduce-transparency", fallback: "Reduce Transparency is on in System Settings, so every window is opaque. Your opacity applies again when it's off.")
                    : Words.text("settings-appearance-opaque-increase-contrast", fallback: "Increase Contrast is on in System Settings, so every window is opaque. Your opacity applies again when it's off.")
            )
            .padding(.vertical, Space._1)
        }
    }
}

/// Below 95 % without the blur the desktop shows sharp behind the text: a warning with the button that turns blur on.
private struct BlurWarning: View {
    let model: SettingsModel
    let entry: SettingsSchema.Entry

    var body: some View {
        if model.host.blurIsUnsafe(opacity: model.value("opacity").number ?? 1, blur: model.value("blur").bool ?? true) {
            Banner(
                kind: .warning,
                text: Words.text("settings-appearance-blur-off-warning", fallback: "Without blur the desktop shows sharp behind the text: below 95 % opacity it may not read.")
            ) {
                if !RowExtras.projectSets(entry, model) {
                    ActionButton(title: Words.text("settings-appearance-turn-on-blur", fallback: "Turn on blur"), symbol: nil) {
                        model.set("blur", .bool(true))
                    }
                }
            }
            .padding(.vertical, Space._1)
        }
    }
}

/// The switch can be on and show no glass while accessibility holds it back: say why.
private struct GlassNote: View {
    let model: SettingsModel
    @Environment(\.reduceTransparency) private var reduceTransparency
    @Environment(\.increaseContrast) private var increaseContrast

    var body: some View {
        if (model.value("liquid_glass").bool ?? false) && (reduceTransparency || increaseContrast) {
            Banner(
                kind: .info,
                text: Words.text("settings-appearance-glass-held-by-accessibility", fallback: "Reduce Transparency or Increase Contrast is on in System Settings, so every surface stays painted. The glass returns when it's off.")
            )
            .padding(.vertical, Space._1)
        }
    }
}

/// The theme's own colours in the order GPUI offers them, each once, then the colour already chosen when it is none of them.
@MainActor func offeredSwatches(ansi order: [Int], skip: String?, chosen: String?) -> [String] {
    let palette = ThemeStore.shared.theme.tokens().terminal.ansi
    var colors: [String] = []
    for hex in order.compactMap({ palette.indices.contains($0) ? palette[$0].hexOpaque : nil }) + (chosen.map { [$0] } ?? []) {
        if hex != skip, !colors.contains(hex) { colors.append(hex) }
    }
    return colors
}

extension RGBA {
    /// `#rrggbb` whatever the alpha (a swatch is the colour itself).
    var hexOpaque: String { RGBA(red, green, blue, 1).hex }
}

/// Primary colour, theme's own switched off: the theme's ANSI colours to pick from (the system's well is the control's).
private struct PrimarySwatches: View {
    let model: SettingsModel
    let entry: SettingsSchema.Entry

    var body: some View {
        let usesTheme = model.value("theme_primary").bool ?? true
        if !usesTheme, !RowExtras.projectSets(entry, model) {
            let raw = (model.value("primary_color").string ?? "").trimmingCharacters(in: .whitespaces).lowercased()
            let chosen = Color(hex: raw) == nil ? nil : raw
            let themePrimary = ThemeStore.shared.theme.tokens().rgba(named: "primary")?.hexOpaque
            SwatchRow(colors: offeredSwatches(ansi: [2, 5, 1, 3, 6, 4], skip: themePrimary, chosen: chosen), chosen: chosen) {
                model.set("primary_color", .string($0))
                model.set("theme_primary", .bool(false))
            }
            .padding(.vertical, Space._1)
        }
    }
}

/// Branch colour: Custom: the theme's ANSI colours, the one picked, and a well for any other. A pick is written with
/// the choice of Custom, as `Settings::choose_branch_color` does.
private struct BranchSwatches: View {
    let model: SettingsModel
    let entry: SettingsSchema.Entry

    var body: some View {
        if model.value("branch_color").string == "custom", !RowExtras.projectSets(entry, model) {
            let text = (model.value("branch_color_custom").string ?? "").trimmingCharacters(in: .whitespaces).lowercased()
            let chosen = Color(hex: text) == nil ? nil : text
            SwatchRow(colors: offeredSwatches(ansi: [5, 1, 3, 2, 6, 4], skip: nil, chosen: chosen), chosen: chosen, pick: pick) {
                ColorPicker(
                    entry.label,
                    selection: Binding(get: { Color(hex: chosen ?? "") ?? .accentColor }, set: { pick($0.hexString) }),
                    supportsOpacity: false
                )
                .labelsHidden()
            }
            .padding(.vertical, Space._1)
        }
    }

    private func pick(_ hex: String) {
        model.set("branch_color_custom", .string(hex))
        model.set("branch_color", .string("custom"))
    }
}

/// A row of swatches, the chosen one ringed; `extra` follows (a colour well).
struct SwatchRow<Extra: View>: View {
    let colors: [String]
    let chosen: String?
    let pick: (String) -> Void
    @ViewBuilder var extra: Extra
    @Environment(\.tokens) private var tokens

    var body: some View {
        HStack(spacing: Space._1) {
            ForEach(colors, id: \.self) { hex in
                Button { pick(hex) } label: {
                    RoundedRectangle(cornerRadius: tokens.metrics.radiusSm)
                        .fill(Color(hex: hex) ?? .clear)
                        .frame(width: SettingsMetric.swatchSize, height: SettingsMetric.swatchSize)
                        .overlay {
                            // The chosen swatch's mark (a selection, not the keyboard's ring).
                            if hex == chosen {
                                RoundedRectangle(cornerRadius: tokens.metrics.radiusSm).stroke(tokens.textPrimary, lineWidth: tokens.metrics.focusRingWidth)
                            }
                        }
                }
                .buttonStyle(.plain)
                .accessibilityLabel(hex)
                .accessibilityAddTraits(hex == chosen ? .isSelected : [])
            }
            extra
        }
    }
}

extension SwatchRow where Extra == EmptyView {
    init(colors: [String], chosen: String?, pick: @escaping (String) -> Void) {
        self.init(colors: colors, chosen: chosen, pick: pick) { EmptyView() }
    }
}
