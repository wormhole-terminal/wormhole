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
                Text("\(state.name)/.wormhole").font(TextRole.body.font).foregroundStyle(tokens.textPrimary).lineLimit(1)
                Spacer(minLength: 0)
                StatusBadge(text: state.status.badge, color: badgeColor)
            }
            .padding(.horizontal, PanelMetrics.inset).frame(height: Metric.band)
            .accessibilityElement(children: .combine)
            if state.asksForTrust { TrustNotice(store: store, state: state) }
            // Notes sync reads and writes the repository's `notes.md`, so it waits for trust too (`notes_sync.rs`).
            if state.status != .trusted, state.features.contains(where: { $0.feature == .notes && $0.on }) {
                problem(panelText("project-notes-waits", "Notes sync: waits for the project to be trusted; its notes.md is neither read nor written until then"), color: tokens.statusDangerText)
            }
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
        Text(text).font(TextRole.caption.font).foregroundStyle(color)
            .padding(.horizontal, PanelMetrics.inset).padding(.vertical, Space._1)
            .frame(maxWidth: .infinity, alignment: .leading)
    }
}
