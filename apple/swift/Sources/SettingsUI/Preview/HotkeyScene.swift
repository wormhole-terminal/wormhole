// Hotkey's scene (`hotkey_preview.rs`): the shortcut as key caps and what it opens.

import DesignSystem
import SettingsModel
import SwiftUI
import WordsModel

struct HotkeyScene: View {
    let model: SettingsModel
    var body: some View {
        let t = PreviewTheme.tokens(model)
        let spec = model.value("hotkey").string ?? ""
        let opensPanel = (model.value("hotkey_action").string ?? "quick_panel") == "quick_panel"
        VStack(spacing: Space._3) {
            HStack(spacing: Space._1) {
                ForEach(Array(KeySpecText.caps(spec).enumerated()), id: \.offset) { _, cap in
                    Text(cap).textRole(.heading, weight: .medium).frame(minWidth: Metric.control + 8, minHeight: Metric.control + 8)
                        .background(t.controlSelected, in: RoundedRectangle(cornerRadius: t.metrics.radiusChip))
                }
                if spec.isEmpty { Text(SettingsText.recorderNone).foregroundStyle(t.textFaint) }
            }
            Image(systemName: "arrow.down").foregroundStyle(t.textFaint)
            Label(
                opensPanel
                    ? Words.text("settings-hotkey-quick-panel", fallback: "Quick panel")
                    : Words.text("settings-hotkey-show-or-hide-wormhole", fallback: "Show or hide Wormhole"),
                systemImage: opensPanel ? "rectangle.and.text.magnifyingglass" : "macwindow")
                .textRole(.body).foregroundStyle(t.textPrimary)
        }
        .foregroundStyle(t.textPrimary)
    }
}
