// The Changed page (`changed.rs`): every setting that differs from its default, under the page it belongs to,
// each with its reset arrow always shown at the right; Reset All… (confirmed) puts every one back. It holds no
// settings of its own: it is the other pages' rows, found by what differs.

import DesignSystem
import SettingsModel
import SwiftUI
import WordsModel

struct ChangedPage: View {
    let model: SettingsModel
    @State private var confirming = false
    @Environment(\.tokens) private var tokens

    var body: some View {
        let changed = model.changedEntries
        let sections = model.schema.sections.filter { section in changed.contains { $0.section.id == section.id } }
        PageScaffold(title: SettingsText.changedTitle, intro: SettingsText.changedIntro, action: {
            if !changed.isEmpty {
                ActionButton(title: SettingsText.resetAll, symbol: "arrow.counterclockwise", isDestructive: true) { confirming = true }
            }
        }) {
            if changed.isEmpty {
                VStack(spacing: Space._1) {
                    Text(SettingsText.noneChanged).font(.system(size: TextRole.heading.size, weight: .semibold))
                    Text(SettingsText.noneChangedDetail).foregroundStyle(tokens.textMuted)
                }
                .frame(maxWidth: .infinity).padding(.vertical, Space._6 * 2)
                .accessibilityElement(children: .combine)
            }
            ForEach(sections) { section in
                SettingGroup(title: section.title) {
                    ForEach(changed.filter { $0.section.id == section.id }) {
                        EntryRowView(model: model, entry: $0.entry, resetAlways: true)
                    }
                }
            }
        }
        .confirmationDialog(SettingsText.resetAll, isPresented: $confirming) {
            Button(SettingsText.resetAll, role: .destructive) { model.resetAll() }
        } message: {
            Text(Words.text("settings-changed-reset-all-confirm", fallback: "Every changed setting goes back to its default."))
        }
    }
}
