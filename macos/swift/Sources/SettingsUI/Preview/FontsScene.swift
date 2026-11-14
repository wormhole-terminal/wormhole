// Fonts' scene (`draw_specimen`): a prompt, a result and code with the pairs ligatures join, set in the terminal
// font at its size and line height on the theme's terminal colours. While a font picker is open it shows the font
// being looked at, and ligatures follow the setting as the terminal's do.

import DesignSystem
import FontPickerUI
import SettingsModel
import SwiftUI
import WordsModel

struct FontsScene: View {
    let model: SettingsModel
    /// The slot the specimen is set in, Terminal | Code | Interface (`window.specimen`).
    @State private var slot = "terminal"
    @Environment(\.tokens) private var tokens

    var body: some View {
        let t = PreviewTheme.tokens(model)
        let family = model.fontPreview?.family ?? Self.familyOf(slot, model)
        let size = model.fontPreview?.size ?? Self.sizeOf(slot, model)
        // The terminal's own line height; the other slots are set at 1.4 (as GPUI's specimen is).
        let height = slot == "terminal" ? model.value("line_height").number ?? 1.3 : 1.4
        let font = FontFace.font(
            family, size: size, ligatures: model.value("font_ligatures").bool ?? true, monospacedFallback: true)
        VStack(alignment: .leading, spacing: Space._2) {
            Picker("", selection: $slot) {
                Text(Words.text("settings-appearance-group-terminal", fallback: "Terminal")).tag("terminal")
                Text(Words.text("settings-appearance-group-code", fallback: "Code")).tag("code")
                Text(Words.text("settings-appearance-group-interface", fallback: "Interface")).tag("interface")
            }
            .labelsHidden().pickerStyle(.segmented).fixedSize()
            // A font looked at in a slot's picker shows in that slot's tab, with its size and line height.
            .onChange(of: model.fontPreview?.slot) { _, picked in if let picked { slot = picked } }
            VStack(alignment: .leading, spacing: max(0, size * (height - 1.2))) {
                Text("$ cargo test --release").foregroundStyle(t.terminal.foreground.color) // text: sample terminal output
                Text("test result: ok. 412 passed").foregroundStyle(t.terminal.ansi[2].color) // text: sample terminal output
                Text("fn map(x: &str) -> Option<u8> {").foregroundStyle(t.terminal.ansi[4].color) // text: sample code
                Text("    x.parse().ok() // != == => -> >=").foregroundStyle(t.terminal.foreground.color) // text: sample code
                Text("}").foregroundStyle(t.terminal.ansi[4].color)
            }
            .font(font)
            .padding(Space._4)
            .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
            .background(t.terminal.background.color, in: RoundedRectangle(cornerRadius: t.metrics.radius))
            Text(Self.facts(family: family, size: size, theme: PreviewTheme.name(model), slot: slot, model: model)).textRole(.caption)
                .foregroundStyle(tokens.textMuted).lineLimit(1).truncationMode(.tail)
        }
    }

    /// The slot's family: an empty code font follows the terminal's, an empty interface font is the system's. These three are the
    /// control folder's `settings-page` specimen lines too.
    static func familyOf(_ slot: String, _ model: SettingsModel) -> String {
        let terminal = model.value("font_family").string ?? ""
        switch slot {
        case "code": return model.value("code_font_family").string.flatMap { $0.isEmpty ? nil : $0 } ?? terminal
        case "interface": return model.value("ui_font_family").string.flatMap { $0.isEmpty ? nil : $0 } ?? FontFace.system
        default: return terminal
        }
    }

    static func sizeOf(_ slot: String, _ model: SettingsModel) -> Double {
        switch slot {
        case "code": model.value("code_font_size").number ?? 13
        case "interface": model.value("ui_font_size").number ?? 13
        default: model.value("font_size").number ?? 14
        }
    }

    /// What the specimen is set in, on one line: the family and size, the terminal's line height, the ligatures, the theme.
    static func facts(family: String, size: Double, theme: String, slot: String, model: SettingsModel) -> String {
        let shown = family == FontFace.system ? "System font" : (family.isEmpty ? "JetBrains Mono" : family) // text: the system font's name and the bundled family's
        var parts = [Words.text("settings-fonts-specimen-size", ["family": .text(shown), "size": .text(String(format: "%.0f", size))], fallback: "\(shown) \(String(format: "%.0f", size)) pt")]
        if slot == "terminal" {
            let value = String(format: "%.2f", model.value("line_height").number ?? 1.3)
            parts.append(Words.text("settings-fonts-specimen-line-height", ["value": .text(value)], fallback: "line height \(value)"))
        }
        if slot != "interface" {
            parts.append((model.value("font_ligatures").bool ?? true)
                ? Words.text("settings-fonts-ligatures-on", fallback: "ligatures on") : Words.text("settings-fonts-ligatures-off", fallback: "ligatures off"))
        }
        parts.append(Words.text("settings-fonts-specimen-colours", ["theme": .text(theme)], fallback: "\(theme) colours"))
        return parts.joined(separator: "   ")
    }
}
