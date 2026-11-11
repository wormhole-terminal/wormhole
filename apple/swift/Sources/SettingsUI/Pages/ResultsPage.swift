// The results of a search (`render_results`): the hits grouped by their section, each row live and its found
// words marked, and a link on each group to open the page "in context". Nothing found says so.

import DesignSystem
import SettingsModel
import SwiftUI
import WordsModel

struct ResultsPage: View {
    let model: SettingsModel

    var body: some View {
        let hits = model.hits
        let sections = model.schema.sections.filter { section in hits.contains { $0.section == section.id } }
        PageScaffold(title: SettingsText.results, intro: summary(hits, sections.count), action: { EscHint() }) {
            ForEach(sections) { section in
                SettingGroup(title: section.title) {
                    LinkButton(title: Words.text("settings-window-open-section", fallback: "Open \(section.title)"), goes: true) {
                        model.show(section.id)
                        model.query = ""
                    }
                } rows: {
                    ForEach(hits.filter { $0.section == section.id }) { hit in
                        EntryRowView(model: model, entry: hit.entry, found: hit)
                    }
                }
            }
        }
    }

    private func summary(_ hits: [SearchHit], _ sections: Int) -> String {
        hits.isEmpty ? SettingsText.noResults : SettingsText.resultsSummary(hits: hits.count, sections: sections, query: model.query)
    }
}

private struct EscHint: View {
    @Environment(\.tokens) private var tokens
    var body: some View {
        HStack(spacing: Space._2) {
            Text("esc").textRole(.caption, weight: .medium).padding(.horizontal, Space._1)
                .background(tokens.controlBackground, in: RoundedRectangle(cornerRadius: tokens.metrics.radiusChip))
            Text(SettingsText.escClears).textRole(.caption).foregroundStyle(tokens.textMuted)
        }
    }
}
