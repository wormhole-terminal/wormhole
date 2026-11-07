// The Terminals list scene (`draw_sidebar_preview`): the three densities side by side as the sidebar draws them,
// the current one named in the primary and ringed, so choosing is comparing. With `withAgents` the rows show
// agents at work (Coding agents' page). Regular rows carry a second line; Compact and Dense one line.

import DesignSystem
import SettingsModel
import SwiftUI

struct SidebarScene: View {
    let model: SettingsModel
    let withAgents: Bool
    @Environment(\.tokens) private var tokens

    private struct Sample { let name: String, detail: String, state: AgentState }
    enum AgentState { case working, needs, done }
    private static let samples = [
        Sample(name: "Rate limits", detail: "Working · 4m", state: .working),
        Sample(name: "Flaky login test", detail: "Needs input · Allow edit?", state: .needs),
        Sample(name: "Release notes", detail: "Finished · 1m", state: .done),
    ]
    /// Coding agents' page: the list as agents drive it, in the density in force.
    private static let agentSamples = [
        Sample(name: "Claude Code", detail: "Working · 4m", state: .working),
        Sample(name: "Codex", detail: "Needs input · Allow edit?", state: .needs),
        Sample(name: "Gemini CLI", detail: "Finished · 1m", state: .done),
    ]
    private static let densities: [(value: String, label: String, row: CGFloat, tile: CGFloat)] = [
        ("regular", "Regular", Metric.rowTwoLine + 4, Metric.tile), ("compact", "Compact", 32, Metric.tileMini),
        ("dense", "Dense", 26, Metric.tileSmall),
    ]

    var body: some View {
        let t = PreviewTheme.tokens(model)
        let current = model.value("density").string ?? "regular"
        HStack(alignment: .top, spacing: Space._3) {
            ForEach(Self.densities.filter { !withAgents || $0.value == current }, id: \.value) { density in
                let isCurrent = density.value == current
                VStack(alignment: .leading, spacing: Space._1) {
                    Text(density.label).font(.system(size: TextRole.label.size)).foregroundStyle(isCurrent ? t.primary : t.textMuted)
                    cell(t, density, isCurrent)
                }
                .frame(maxWidth: .infinity)
            }
        }
        .motion(Motion.selection, value: current)
    }

    private func cell(_ t: ThemeTokens, _ d: (value: String, label: String, row: CGFloat, tile: CGFloat), _ isCurrent: Bool) -> some View {
        VStack(alignment: .leading, spacing: 2) {
            VStack(alignment: .leading, spacing: 0) {
                Text("Release 1.2").font(.system(size: 12, weight: .semibold)).foregroundStyle(t.textPrimary)
                if d.value == "regular" { Text("~/Projects/wormhole").font(.system(size: 11)).foregroundStyle(t.textMuted) }
            }
            .padding(.horizontal, Space._2).padding(.vertical, Space._1)
            ForEach(Array((withAgents ? Self.agentSamples : Self.samples).enumerated()), id: \.offset) { index, sample in
                HStack(spacing: Space._2) {
                    AgentTile(tokens: t, size: d.tile, badge: sample.state == .needs ? t.agentAttention : (sample.state == .done ? t.agentDone : nil))
                    VStack(alignment: .leading, spacing: 0) {
                        Text(sample.name).font(.system(size: 12, weight: index == 0 ? .semibold : .regular)).lineLimit(1)
                            .foregroundStyle(index == 0 ? t.sidebarRowSelectedText : t.textPrimary)
                        if d.value == "regular" {
                            Text(sample.detail).font(.system(size: 11)).lineLimit(1)
                                .foregroundStyle(sample.state == .needs ? t.agentAttentionText : (index == 0 ? t.sidebarRowSelectedMuted : t.textMuted))
                        }
                    }
                    Spacer(minLength: 0)
                }
                .padding(.horizontal, Space._2).frame(height: d.row)
                .background(index == 0 ? t.sidebarRowSelected : .clear, in: RoundedRectangle(cornerRadius: Radius.row))
            }
            Spacer(minLength: 0)
        }
        .padding(Space._1).frame(maxHeight: .infinity, alignment: .top)
        .background(t.sidebarBackground, in: RoundedRectangle(cornerRadius: t.metrics.radiusLg))
        .overlay(RoundedRectangle(cornerRadius: t.metrics.radiusLg).strokeBorder(isCurrent ? t.primary : .clear, lineWidth: 2))
        .clipped()
    }
}
