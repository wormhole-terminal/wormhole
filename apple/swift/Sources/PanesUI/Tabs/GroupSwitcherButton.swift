// The group switcher in the bar: the current group's name and up-down chevrons, an amber dot on its corner
// while another group has a terminal that needs you (ringed by the bar's colour so it reads as a dot).

import DesignSystem
import PanesModel
import SwiftUI

struct GroupSwitcherButton: View {
    let label: String
    let alert: Bool
    @Binding var open: Bool
    @State private var hovering = false
    @Environment(\.tokens) private var tokens

    var body: some View {
        Button { open.toggle() } label: {
            HStack(spacing: Space._2) {
                Text(label).font(TextRole.label.font).foregroundStyle(tokens.textPrimary).lineLimit(1)
                    .frame(maxWidth: TabMetrics.switcherName, alignment: .leading).fixedSize()
                Image(systemName: "chevron.up.chevron.down").font(.system(size: Metric.chevron, weight: .semibold))
                    .foregroundStyle(tokens.textMuted)
            }
            .padding(.horizontal, Space._2)
            .frame(height: TabMetrics.switcherHeight)
            .background(hovering || open ? tokens.bandHover : .clear, in: RoundedRectangle(cornerRadius: tokens.metrics.radius))
            .overlay(alignment: .topTrailing) {
                if alert {
                    Circle().fill(tokens.agentAttention).frame(width: Metric.statusDot - 2, height: Metric.statusDot - 2)
                        .padding(TabMetrics.alertRing)
                        .background(tokens.surfacePanel, in: Circle())
                        .transition(.scale.combined(with: .opacity))
                }
            }
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .onHover { hovering = $0 }
        .accessibilityLabel(PanesText.switcherLabel(label, alert: alert))
    }
}
