// The terminal's tile and name, and what it says it is doing after them: "working · 4m" in the state's
// colour (attention and failure stand out), when the status bar doesn't say it. A terminal hidden from the
// programs that control Wormhole wears a lock. The name is what gives way in a narrow row: cut at its end.

import DesignSystem
import PanesModel
import SidebarUI
import SwiftUI
import WorkspaceModel

struct TerminalCrumb: View {
    let plan: TitlePlan
    /// The row's own tile (the agent's logo with its live status), when the window gives one.
    var tile: TerminalStatusTile?
    @Environment(\.tokens) private var tokens

    var body: some View {
        let facts = plan.facts
        HStack(spacing: TitleMetrics.crumbGap) {
            if let tile { tile } else { TerminalTile(activity: facts.activity, size: TitleMetrics.tile) }
            AtMostWidth(width: plan.fit.name) {
                Text(facts.label).font(tokens.font(.label).weight(.medium)).foregroundStyle(tokens.textPrimary)
                    .lineLimit(1).truncationMode(.tail)
                    .contentTransition(.interpolate)
            }
            if facts.hiddenFromAgents {
                Image(systemName: "lock").font(.system(size: Metric.glyph)).foregroundStyle(tokens.textMuted)
                    .help(PanesText.hiddenFromAgents)
            }
        }
        .accessibilityElement(children: .combine)
    }
}

struct StatusWords: View {
    let words: String
    let activity: Activity
    @Environment(\.tokens) private var tokens

    var body: some View {
        Text(words).font(tokens.font(.caption)).foregroundStyle(colour)
            .lineLimit(1).fixedSize()
            .transition(.opacity)
    }

    private var colour: Color {
        switch activity {
        case .needsInput: tokens.agentAttentionText
        case .failed: tokens.statusDangerText
        default: tokens.textMuted
        }
    }
}
