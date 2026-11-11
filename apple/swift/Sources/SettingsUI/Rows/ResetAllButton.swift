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
    var body: some View {
        let count = model.changedKeyCount
        ActionButton(title: label, symbol: icon, isDestructive: true, frameId: "reset-all") { ask(count) }
            .disabled(count == 0)
            .help(count == 0
                ? Words.text("settings-advanced-every-setting-is-at-its-default", fallback: "Every setting is at its default")
                : Words.text("settings-advanced-asks-first", fallback: "Asks first"))
    }

    private func ask(_ count: Int) {
        Task {
            let reset = Words.text("settings-advanced-reset", fallback: "Reset")
            if await SettingsQuestion.confirm(title: label, detail: model.host.resetQuestion(count: count), confirm: reset) { model.resetAll() }
        }
    }
}
