// What a coding agent runs and how full its context window is: the model's name (at most 96 pt) and a
// 28 × 4 meter with the share used, whichever the agent reported, in muted caption type. The meter fills in
// the primary, or the warning colour from 80 % on, which is early enough to compact.

import DesignSystem
import PanesModel
import SwiftUI

struct GaugeChip: View {
    let gauge: GaugeFact
    @Environment(\.tokens) private var tokens

    /// The meter's height (`METER_H`).
    private static let meterHeight = 4.0

    var body: some View {
        HStack(spacing: TitleMetrics.extraGap) {
            if let model = gauge.model {
                Text(model).lineLimit(1).truncationMode(.tail).frame(maxWidth: TitleMetrics.gaugeModelMax, alignment: .leading).fixedSize()
            }
            if let percent = gauge.percent {
                meter(share: (gauge.context ?? 0) / 100)
                Text("\(percent)%").monospacedDigit()
            }
        }
        .font(tokens.font(.caption).weight(.medium))
        .foregroundStyle(tokens.textMuted)
        .padding(.horizontal, TitleMetrics.extraPad)
        .frame(height: TitleMetrics.chipHeight)
        .help(gauge.words)
        .accessibilityElement(children: .ignore)
        .accessibilityLabel(gauge.words)
    }

    private func meter(share: Double) -> some View {
        let shape = RoundedRectangle(cornerRadius: tokens.metrics.radiusSm)
        return shape.fill(tokens.border)
            .overlay(alignment: .leading) {
                shape.fill(gauge.warns ? tokens.statusWarning : tokens.primary)
                    .frame(width: TitleMetrics.meter * min(max(share, 0), 1))
            }
            .frame(width: TitleMetrics.meter, height: Self.meterHeight)
            .clipShape(shape)
            .animation(Motion.selection, value: share)
    }
}
