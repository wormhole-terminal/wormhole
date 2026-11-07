// A menu that looks like a band's icon button (`BandButton`): the glyph alone, muted until the pointer is over it. The
// menu itself is the system's.

import DesignSystem
import SwiftUI

struct BandMenu<Items: View>: View {
    let symbol: String
    let label: String
    @ViewBuilder let items: Items
    @State private var hovering = false
    @Environment(\.tokens) private var tokens

    var body: some View {
        Menu {
            items
        } label: {
            Image(systemName: symbol)
                .font(.system(size: PanelMetrics.bandGlyph, weight: .regular))
                .foregroundStyle(hovering ? tokens.textPrimary : tokens.textMuted)
                .frame(width: PanelMetrics.bandButton, height: PanelMetrics.bandButton)
                .background(hovering ? tokens.bandHover : .clear, in: RoundedRectangle(cornerRadius: tokens.metrics.radiusSm))
                .frame(width: Metric.hitMin, height: Metric.hitMin)
                .contentShape(Rectangle())
        }
        .menuStyle(.button).buttonStyle(.plain).menuIndicator(.hidden)
        .fixedSize()
        .padding(.horizontal, -(Metric.hitMin - PanelMetrics.bandButton) / 2)
        .onHover { hovering = $0 }
        .help(label)
        .accessibilityLabel(label)
    }
}
