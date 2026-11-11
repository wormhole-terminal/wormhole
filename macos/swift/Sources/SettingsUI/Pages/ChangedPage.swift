// The Changed page (`changed.rs`): every setting that differs from its default, under the page it belongs to,
// each with its reset arrow always shown at the right; Reset All… (confirmed) puts every one back. It holds no
// settings of its own: it is the other pages' rows, found by what differs.

import DesignSystem
import SettingsModel
import SwiftUI
import WordsModel

/// "7 settings changed" and what is left, with the two buttons that act on all of them (`changed_summary`); with none, the card
/// says so and has no buttons.
private struct ChangedSummary: View {
    let title: String
    let detail: String
    let showsButtons: Bool
    let copy: () -> String
    let resetAll: () -> Void
    @State private var copied = false
    @Environment(\.tokens) private var tokens

    var body: some View {
        HStack(spacing: Space._3) {
            Image(Icon.reset).foregroundStyle(tokens.sidebarRowSelectedText)
                .frame(width: Metric.tile, height: Metric.tile)
                .background(tokens.sidebarRowSelected, in: RoundedRectangle(cornerRadius: tokens.metrics.radius))
                .accessibilityHidden(true)
            VStack(alignment: .leading, spacing: 0) {
                Text(title).textRole(.body, weight: .semibold).foregroundStyle(tokens.textPrimary)
                Text(detail).textRole(.caption).foregroundStyle(tokens.textMuted)
            }
            Spacer(minLength: Space._3)
            if showsButtons {
                ActionButton(title: copied ? SettingsText.copied : SettingsText.copyJSON, symbol: Icon.copy.rawValue, frameId: "changed.copy") {
                    Pasteboard.setText(copy())
                    Announcements.post(SettingsText.copied)
                    copied = true
                }
                .task(id: copied) {
                    guard copied else { return }
                    // one-shot: "Copied" goes back to the button's words
                    try? await Task.sleep(for: .seconds(1.5))
                    copied = false
                }
                ActionButton(title: SettingsText.resetAll, symbol: nil, isDestructive: true, frameId: "changed.reset-all", action: resetAll)
            }
        }
        .padding(.vertical, Space._3)
        .accessibilityElement(children: .contain)
        .accessibilityLabel(title)
    }
}

struct ChangedPage: View {
    let model: SettingsModel
    @Environment(\.tokens) private var tokens

    var body: some View {
        let changed = model.changedEntries
        let sections = model.schema.sections.filter { section in changed.contains { $0.section.id == section.id } }
        PageScaffold(title: SettingsText.changedTitle, intro: SettingsText.changedIntro, action: { EmptyView() }) {
            SettingGroup(title: "") {
                ChangedSummary(
                    title: changed.isEmpty ? SettingsText.noneChanged : model.host.changedSummary(count: changed.count),
                    detail: changed.isEmpty ? SettingsText.noneChangedDetail : SettingsText.changedSummaryDetail,
                    showsButtons: !changed.isEmpty, copy: model.host.changedJSON, resetAll: resetAll)
            }
            ForEach(sections) { section in
                SettingGroup(title: section.title) {
                    ForEach(changed.filter { $0.section.id == section.id }) {
                        EntryRowView(model: model, entry: $0.entry, resetAlways: true)
                    }
                }
            }
        }
    }

    /// Reset All…: asks first, then puts back the settings this page lists as they are when the answer comes.
    private func resetAll() {
        Task {
            let count = model.changedKeyCount
            let reset = Words.text("settings-advanced-reset", fallback: "Reset")
            if await SettingsQuestion.confirm(title: SettingsText.resetAll, detail: model.host.resetQuestion(count: count), confirm: reset) { model.resetAll() }
        }
    }
}
