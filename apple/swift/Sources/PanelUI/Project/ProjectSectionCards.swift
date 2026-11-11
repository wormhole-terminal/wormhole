// The Project tab's sections, each a card with the same head as Notes and Changes, open, with its count: the per-feature
// switches ("Use from this project", for you only), the actions (a click runs one), the prompts (paste one, or copy it
// into a note to edit first) and the terminal layout with the button that opens it.

import DesignSystem
import PanelModel
import SwiftUI

struct ProjectFeaturesCard: View {
    let store: ProjectStore
    let state: ProjectState
    @Environment(\.tokens) private var tokens

    var body: some View {
        PanelCard {
            CardBand(title: panelText("project-use", "Use from this project"))
            ForEach(state.features) { row in
                TwoLineRow(title: row.feature.label, detail: row.hint, detailWarns: row.warns) {
                    Image(systemName: row.feature.symbol).font(.system(size: Metric.glyph)).foregroundStyle(tokens.textMuted)
                } trailing: {
                    Toggle(row.feature.label, isOn: Binding(get: { row.on }, set: { store.setFeature(row.feature, on: $0) }))
                        .labelsHidden().toggleStyle(.switch).controlSize(.small).tint(tokens.primary)
                }
            }
        }
    }
}

struct ProjectActionsCard: View {
    let store: ProjectStore
    let actions: [ProjectAction]
    @Environment(\.tokens) private var tokens

    var body: some View {
        PanelCard {
            CardBand(title: panelText("project-actions", "Actions"), count: CountText.grouped(actions.count))
            ForEach(actions) { action in
                TwoLineRow(title: action.title, detail: action.detail, action: { store.run(action) }) {
                    PanelIcon("play.fill", size: 11).foregroundStyle(tokens.primary)
                } trailing: {
                    if let key = action.key { Text(key).font(TextRole.caption.font).foregroundStyle(tokens.textFaint) }
                }
                .accessibilityLabel(action.title)
            }
        }
    }
}

struct ProjectPromptsCard: View {
    let store: ProjectStore
    let notes: NotesStore
    let prompts: [ProjectPrompt]
    @Environment(\.tokens) private var tokens

    var body: some View {
        PanelCard {
            CardBand(title: panelText("project-prompts", "Prompts"), count: CountText.grouped(prompts.count))
            ForEach(prompts) { prompt in
                TwoLineRow(
                    title: prompt.title, detail: prompt.detail,
                    actions: [
                        .init(symbol: "arrow.up.right", label: panelText("project-prompt-paste", "Paste into the terminal")) { store.send(prompt) },
                        .init(symbol: "doc.badge.plus", label: panelText("project-prompt-note", "Copy into a new note")) { store.copyToNote(prompt, in: notes) },
                    ]
                ) {
                    PanelIcon("doc.text", size: Metric.glyph).foregroundStyle(tokens.textMuted)
                }
            }
        }
    }
}

struct ProjectLayoutCard: View {
    let store: ProjectStore
    let names: [String]
    @Environment(\.tokens) private var tokens

    var body: some View {
        PanelCard {
            CardBand(title: panelText("project-layout", "Terminal layout"), count: CountText.grouped(names.count))
            HStack(spacing: PanelMetrics.gap) {
                Text(names.joined(separator: " \u{b7} ")).font(TextRole.caption.font).foregroundStyle(tokens.textMuted).lineLimit(1)
                    .frame(maxWidth: .infinity, alignment: .leading)
                NoticeButton(title: panelText("project-open-layout", "Open terminals"), primary: true, action: store.openLayout)
            }
            .padding(.leading, PanelMetrics.inset + Metric.indent).padding(.trailing, PanelMetrics.inset).frame(height: Metric.titleRow)
        }
    }
}
