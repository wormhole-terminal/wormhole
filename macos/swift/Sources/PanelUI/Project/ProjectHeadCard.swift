// The folder and its trust, the header the sections hang from: the project folder's name with its trust as a badge, the
// notice that asks for trust (or one quiet line with Revoke once it has it), and what could not load.

import DesignSystem
import PanelModel
import SwiftUI

struct ProjectHeadCard: View {
    let store: ProjectStore
    let state: ProjectState
    @Environment(\.tokens) private var tokens

    var body: some View {
        PanelCard {
            HStack(spacing: PanelMetrics.gap) {
                FileIcon(kind: .folder("", open: false), tint: tokens.textMuted)
                Text("\(state.name)/\(state.dir)").font(tokens.font(.body)).foregroundStyle(tokens.textPrimary).lineLimit(1)
                Spacer(minLength: 0)
                StatusBadge(text: state.status.badge, color: badgeColor)
            }
            .padding(.horizontal, PanelMetrics.inset).frame(height: Metric.band)
            .accessibilityElement(children: .combine)
            if state.asksForTrust { TrustNotice(store: store, state: state) }
            // The notes sync says why it waits (untrusted) through the errors, as the hub's does (`notes_sync.rs`).
            ForEach(state.errors, id: \.self) { problem($0, color: tokens.statusDangerText) }
            ForEach(state.warnings, id: \.self) { problem($0, color: tokens.statusWarningText) }
        }
    }

    private var badgeColor: Color {
        switch state.status {
        case .trusted: tokens.statusSuccessText
        case .changed: tokens.statusWarningText
        case .untrusted: tokens.textMuted
        }
    }

    /// A line about something the project couldn't load, in the danger colour; a warning is the warning colour.
    private func problem(_ text: String, color: Color) -> some View {
        Text(text).font(tokens.font(.caption)).foregroundStyle(color)
            .padding(.horizontal, PanelMetrics.inset).padding(.vertical, Space._1)
            .frame(maxWidth: .infinity, alignment: .leading)
    }
}
