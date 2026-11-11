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
                AgentMark(report: report)
                Text(report.name).font(TextRole.body.font.weight(.semibold)).foregroundStyle(tokens.textPrimary).lineLimit(1)
                Spacer(minLength: 0)
                Text(UsageText.updated(report.updated, now: now)).font(TextRole.caption.font.monospacedDigit()).foregroundStyle(tokens.textMuted)
            }
            .padding(.horizontal, PanelMetrics.inset).frame(height: Metric.band)
            if !report.limits.isEmpty {
                SectionLabel(panelText("usage-plan-limits", "Plan usage limits")).padding(.horizontal, PanelMetrics.inset).padding(.bottom, Space._1)
                ForEach(Array(report.limits.enumerated()), id: \.offset) { LimitRow(limit: $1, now: now) }
            }
            if let tokensCount = report.responseTokens {
                VStack(alignment: .leading, spacing: Space._2) {
                    SectionLabel(panelText("usage-latest-response", "Latest model response"))
                    Text(panelText("usage-tokens", ["count": .whole(tokensCount)], "\(String(tokensCount)) tokens")).font(TextRole.body.font.monospacedDigit()).foregroundStyle(tokens.textPrimary)
                    Text(panelText("usage-gemini-note", "Latest report across Gemini terminals. Plan limits are not reported."))
                        .font(TextRole.caption.font).foregroundStyle(tokens.textMuted)
                }
                .padding(.horizontal, PanelMetrics.inset).padding(.vertical, Space._2).frame(maxWidth: .infinity, alignment: .leading)
            }
        }
        .accessibilityElement(children: .contain)
        .accessibilityLabel(report.name)
    }
}

/// "5-hour limit … 25% · resets 34m" over its bar, which turns to the warning colour from 80 % and to the danger one at
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
            // The bar is an 8 pt pill (gpui-component `Progress`, medium), the fill in the level's colour.
            Capsule().fill(tokens.border).frame(height: 8)
                .overlay(alignment: .leading) {
                    GeometryReader { proxy in Capsule().fill(color(share)).frame(width: proxy.size.width * min(max(share, 0), 1)) }
                }
        }
        .padding(.horizontal, PanelMetrics.inset).padding(.vertical, Space._2)
        .accessibilityElement(children: .ignore)
        .accessibilityLabel("\(limit.label), \(words)")
        .accessibilityValue(panelText("usage-spoken-percent", ["percent": .whole(Int(share * 100))], "\(String(Int(share * 100))) percent"))
    }

    private func color(_ share: Double) -> Color {
        switch PlanLimit.level(of: share) {
        case .danger: tokens.statusDanger
        case .warn: tokens.statusWarning
        case .normal: tokens.primary
        }
    }
}

/// An agent's mark (`style::agent_logo`): its bundled logo filled in the muted colour, or its letters in a rounded outline where
/// it has none. The drawing and the letters are the registry's, through the core.
struct AgentMark: View {
    let report: UsageReport
    private static let box: CGFloat = 24
    /// A monogram's corner and letters as shares of its size (`MONOGRAM_CORNER`, `MONOGRAM_TEXT`).
    private static let corner: CGFloat = 3 / 13
    private static let letters: CGFloat = 7 / 13
    @Environment(\.tokens) private var tokens

    var body: some View {
        Group {
            if let logo = report.logo {
                Canvas { canvas, area in
                    let scale = area.width / Self.box
                    var path = Path()
                    for d in logo.paths { path.addPath(SVGPath.path(d), transform: CGAffineTransform(scaleX: scale, y: scale)) }
                    canvas.fill(path, with: .foreground, style: FillStyle(eoFill: logo.evenOdd))
                }
            } else {
                Text(report.monogram).font(.system(size: Metric.glyph * Self.letters, weight: .bold))
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
                    .overlay {
                        RoundedRectangle(cornerRadius: Metric.glyph * Self.corner, style: .continuous)
                            .strokeBorder(lineWidth: tokens.metrics.borderWidth)
                    }
            }
        }
        .foregroundStyle(tokens.textMuted)
        .frame(width: Metric.glyph, height: Metric.glyph)
        .frame(width: Metric.lead, height: Metric.lead)
        .accessibilityHidden(true)
    }
}
