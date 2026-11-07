// Advanced → Reset all settings (`draw_reset`): the danger button, off while every setting is at its default, and a
// question first that says how many go back; the terminals, notes and themes are kept.

import DesignSystem
import SettingsModel
import SwiftUI
import WordsModel

struct ResetAllButton: View {
    let model: SettingsModel
    let label: String
    let icon: String
    @State private var asking = false

    var body: some View {
        let count = model.changedKeyCount
        ActionButton(title: label, symbol: icon, isDestructive: true) { asking = true }
            .disabled(count == 0)
            .help(count == 0
                ? Words.text("settings-advanced-every-setting-is-at-its-default", fallback: "Every setting is at its default")
                : Words.text("settings-advanced-asks-first", fallback: "Asks first"))
            .confirmationDialog(label, isPresented: $asking) {
                Button(Words.text("settings-advanced-reset", fallback: "Reset"), role: .destructive) { model.resetAll() }
            } message: {
                Text(model.host.resetQuestion(count: count))
            }
    }
}
