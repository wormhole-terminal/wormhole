// One commit of the log (`style::commit_row`): two lines, 44 tall like the sidebar's terminals. The chevron in the
// lead column and the subject on the first line; the short hash in the primary (and "merge" for a merge) and the
// author on the second, the age at the right edge. An opened commit is the selected row, its subject semibold in the
// tint's text colour.

import DesignSystem
import PanelModel
import SwiftUI

struct CommitRowView: View {
    let commit: Commit
    let now: Int
    let expanded: Bool
    let action: () -> Void
    @Environment(\.tokens) private var tokens

    var body: some View {
        PanelRow(selected: expanded, action: action) { _ in
            VStack(alignment: .leading, spacing: 0) {
                HStack(spacing: PanelMetrics.gap) {
                    PanelIcon("chevron.right", size: 12).foregroundStyle(muted)
                        .rotationEffect(.degrees(expanded ? 90 : 0)).frame(width: Metric.lead, height: Metric.lead)
                    Text(commit.subject)
                        .font(tokens.font(.body).weight(expanded ? .semibold : .regular))
                        .foregroundStyle(expanded ? tokens.sidebarRowSelectedText : tokens.textPrimary)
                        .lineLimit(1).truncationMode(.tail)
                }
                .frame(height: PanelMetrics.nameLine)
                HStack(spacing: PanelMetrics.gap) {
                    Text(commit.short).font(.system(size: tokens.size(.caption), design: .monospaced))
                        .foregroundStyle(expanded ? tokens.sidebarRowSelectedText : tokens.primary)
                    if commit.merge { Text(panelText("history-merge", "merge")) }
                    Text(commit.author).lineLimit(1)
                    Spacer(minLength: 0)
                    Text(Age.text(seconds: now - commit.time)).monospacedDigit()
                }
                .font(tokens.font(.caption)).foregroundStyle(muted)
                .frame(height: PanelMetrics.metaLine)
                .padding(.leading, Metric.lead + PanelMetrics.gap)
            }
            .padding(.horizontal, PanelMetrics.inset).padding(.vertical, Space._1)
            .frame(height: Metric.rowTwoLine)
        }
        .marked("commit.row")
        .help(commit.subject)
        .accessibilityLabel("\(commit.subject), \(commit.short), \(commit.author)")
        .accessibilityValue(expanded ? panelText("panel-spoken-open", "open") : panelText("panel-spoken-folded", "folded"))
    }

    private var muted: Color { expanded ? tokens.sidebarRowSelectedMuted : tokens.textMuted }
}
