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
        PageScaffold(title: SettingsText.results, intro: hits.isEmpty ? "" : summary(hits, sections.count), action: { EscHint() }) {
            if hits.isEmpty { NothingMatches(model: model) }
            ForEach(sections) { section in
                SettingGroup(title: section.title) {
                    LinkButton(title: Words.text("settings-window-open-section", ["section": .text(section.title)], fallback: "Open \(section.title)"), goes: true) {
                        model.show(section.id)
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
        SettingsText.resultsSummary(hits: hits.count, sections: sections, query: model.query)
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

/// Nothing found: the search icon, what was looked for, and where else the setting may be (`render_no_results`).
private struct NothingMatches: View {
    let model: SettingsModel
    @Environment(\.tokens) private var tokens

    var body: some View {
        VStack(alignment: .leading, spacing: Space._2) {
            Image(Icon.search.rawValue).foregroundStyle(tokens.textMuted)
            Text(Words.text("settings-window-nothing-matches", ["query": .text(model.query)], fallback: "Nothing matches “\(model.query)”"))
                .textRole(.body, weight: .semibold).foregroundStyle(tokens.textPrimary)
            Text(Words.text("settings-window-some-settings-live-only-in-config", fallback: "Some settings live only in config.json, and projects can bring their own actions."))
                .textRole(.caption).foregroundStyle(tokens.textMuted)
            HStack(spacing: Space._2) {
                ActionButton(title: Words.text("settings-window-open-config-json", fallback: "Open config.json"), symbol: nil, frameId: "no-results.config") {
                    model.host.perform(.openConfig)
                }
                LinkButton(title: SettingsText.clearSearch) { model.query = "" }
            }
            .padding(.top, Space._1)
        }
        .padding(Space._4)
        .frame(maxWidth: SettingsMetric.noResultsWidth, alignment: .leading)
        .background(tokens.settingsGroup, in: RoundedRectangle(cornerRadius: tokens.metrics.radiusLg))
        .contrastOutline(RoundedRectangle(cornerRadius: tokens.metrics.radiusLg), \.cardEdge)
        .accessibilityElement(children: .combine)
    }
}
