// One agent: its name and when the numbers came, then a row and a gauge per plan limit; Gemini's latest response in
// tokens in place of limits.

import DesignSystem
import PanelModel
import SwiftUI

struct UsageCard: View {
    let report: UsageReport
    let now: Int
    @Environment(\.tokens) private var tokens

    var body: some View {
        PanelCard {
            HStack(spacing: PanelMetrics.gap) {
                AgentMonogram(name: report.displayName)
                Text(report.displayName).font(TextRole.body.font.weight(.semibold)).foregroundStyle(tokens.textPrimary).lineLimit(1)
                Spacer(minLength: 0)
                Text(UsageText.updated(report.updated, now: now)).font(TextRole.caption.font.monospacedDigit()).foregroundStyle(tokens.textMuted)
            }
            .padding(.horizontal, PanelMetrics.inset).frame(height: Metric.band)
            if !report.limits.isEmpty {
                SectionLabel(panelText("usage-plan-limits", "Plan usage limits")).padding(.horizontal, PanelMetrics.inset).padding(.bottom, Space._1)
                ForEach(report.limits, id: \.label) { LimitRow(limit: $0, now: now) }
            }
            if let tokensCount = report.responseTokens {
                VStack(alignment: .leading, spacing: Space._2) {
                    SectionLabel(panelText("usage-latest-response", "Latest model response"))
                    Text(panelText("usage-tokens", "\(tokensCount.formatted()) tokens")).font(TextRole.body.font.monospacedDigit()).foregroundStyle(tokens.textPrimary)
                    Text(panelText("usage-gemini-note", "Latest report across Gemini terminals. Plan limits are not reported."))
                        .font(TextRole.caption.font).foregroundStyle(tokens.textMuted)
                }
                .padding(.horizontal, PanelMetrics.inset).padding(.vertical, Space._2).frame(maxWidth: .infinity, alignment: .leading)
            }
        }
        .accessibilityElement(children: .contain)
        .accessibilityLabel(report.displayName)
    }
}

/// "5-hour limit … 25% · resets 34m" over its gauge, which turns to the warning colour from 80 % and to the danger one at
/// the limit.
struct LimitRow: View {
    let limit: PlanLimit
    let now: Int
    @Environment(\.tokens) private var tokens

    var body: some View {
        let (share, words) = limit.shown(now: now)
        VStack(alignment: .leading, spacing: Space._2) {
            HStack(alignment: .firstTextBaseline, spacing: PanelMetrics.gap) {
                Text(limit.label).font(TextRole.body.font).foregroundStyle(tokens.textPrimary).lineLimit(1)
                Spacer(minLength: 0)
                Text(words).font(TextRole.caption.font.monospacedDigit()).foregroundStyle(tokens.textMuted)
            }
            Gauge(value: share) { EmptyView() }
                .gaugeStyle(.linearCapacity).tint(color(share))
        }
        .padding(.horizontal, PanelMetrics.inset).padding(.vertical, Space._2)
        .accessibilityElement(children: .ignore)
        .accessibilityLabel("\(limit.label), \(words)")
        .accessibilityValue("\(Int(share * 100)) percent")
    }

    private func color(_ share: Double) -> Color {
        switch PlanLimit.level(of: share) {
        case .danger: tokens.statusDanger
        case .warn: tokens.statusWarning
        case .normal: tokens.primary
        }
    }
}

/// An agent's tile: the first letter on the tile colours, until the brand logos come through the FFI with the registry.
struct AgentMonogram: View {
    let name: String
    @Environment(\.tokens) private var tokens

    var body: some View {
        Text(String(name.prefix(1))).font(.system(size: 10, weight: .bold)).foregroundStyle(tokens.tileMark)
            .frame(width: Metric.tileSmall, height: Metric.tileSmall)
            .background(tokens.tileBackground, in: RoundedRectangle(cornerRadius: tokens.metrics.radiusSm))
            .overlay { RoundedRectangle(cornerRadius: tokens.metrics.radiusSm).strokeBorder(tokens.tileEdge, lineWidth: tokens.metrics.borderWidth) }
            .accessibilityHidden(true)
    }
}
