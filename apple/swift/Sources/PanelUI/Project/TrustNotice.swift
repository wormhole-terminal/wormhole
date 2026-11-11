// Trust (`render_trust`): trusted, one quiet line with Revoke. Otherwise the amber callout: what the project wants to run,
// each line in full with what runs in its own colour, and the one button that approves it, with Touch ID or the password.
// A long list shows its first lines and "View all N…", which opens every line in a window of its own. A project with no
// command to list (only settings, say) gets the callout all the same, saying what trusting applies.

import DesignSystem
import PanelModel
import SwiftUI

struct TrustNotice: View {
    let store: ProjectStore
    let state: ProjectState
    @Environment(\.tokens) private var tokens

    var body: some View {
        if state.status == .trusted { trusted } else { callout }
    }

    private var trusted: some View {
        HStack(spacing: PanelMetrics.gap) {
            PanelIcon("checkmark.shield", size: Metric.glyph).foregroundStyle(tokens.statusSuccessText)
                .frame(width: Metric.lead, height: Metric.lead)
            Text(state.trustedLine).font(TextRole.caption.font).foregroundStyle(tokens.textMuted).lineLimit(2)
                .frame(maxWidth: .infinity, alignment: .leading)
            NoticeButton(title: panelText("project-revoke", "Revoke trust"), action: store.revoke)
        }
        .padding(.horizontal, PanelMetrics.inset).frame(minHeight: Metric.row)
    }

    private var callout: some View {
        let notice = state.notice
        let shown = state.commands.count > ProjectState.linesMax ? ProjectState.linesShown : state.commands.count
        return Callout(kind: .warning, title: notice.title) {
            Text(notice.text).font(TextRole.caption.font).foregroundStyle(tokens.textMuted)
            if !state.commands.isEmpty { TrustLines(lines: Array(state.commands.prefix(shown))) }
            HStack(spacing: PanelMetrics.gap) {
                NoticeButton(title: store.approveTitle, primary: true, busy: store.approving, action: store.approve)
                if shown < state.commands.count {
                    NoticeButton(title: panelText("project-view-all", "View all \(state.commands.count)\u{2026}"), action: store.viewAll)
                }
            }
        }
    }
}

/// What the project would run, one wrapped paragraph per line, never cut off: a cut-off line is where a payload would hide.
struct TrustLines: View {
    let lines: [TrustLine]
    @Environment(\.tokens) private var tokens
    @Environment(\.codeFont) private var codeFont

    var body: some View {
        VStack(alignment: .leading, spacing: Space._1) {
            ForEach(Array(lines.enumerated()), id: \.offset) { _, line in Text(attributed(line)).fixedSize(horizontal: false, vertical: true) }
        }
        .font(codeFont.font(TextRole.caption.size))
        .padding(Space._2).frame(maxWidth: .infinity, alignment: .leading)
        .background(tokens.stateHover, in: RoundedRectangle(cornerRadius: tokens.metrics.radius))
        .textSelection(.enabled)
    }

    /// What runs it muted, what runs in `trust.command`, where after it.
    private func attributed(_ line: TrustLine) -> AttributedString {
        var out = AttributedString(line.what)
        out.foregroundColor = tokens.textMuted
        if !line.runs.isEmpty {
            var runs = AttributedString(" " + line.runs)
            runs.foregroundColor = tokens.trustCommand
            out += runs
        }
        if !line.note.isEmpty {
            var note = AttributedString(" " + line.note)
            note.foregroundColor = tokens.textMuted
            out += note
        }
        return out
    }
}
