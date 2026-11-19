// A two-line row of a card (`style::two_line_row`): a lead glyph, a title in body text and a detail in caption text
// under it, a trailing control or key, and hover actions that take the room at the right end. 44 tall like the
// sidebar's terminals; a row with no detail is one line, 28.

import DesignSystem
import SwiftUI

struct TwoLineRow<Lead: View, Trailing: View>: View {
    let title: String
    var detail: String?
    var detailWarns = false
    var action: (() -> Void)?
    var actions: [RowAction] = []
    @ViewBuilder let lead: Lead
    @ViewBuilder let trailing: Trailing
    @State private var hovering = false
    @Environment(\.tokens) private var tokens

    struct RowAction: Identifiable {
        let symbol: String
        let label: String
        let run: () -> Void
        var id: String { label }
    }

    var body: some View {
        let content = HStack(spacing: PanelMetrics.gap) {
            lead.frame(width: Metric.lead, height: Metric.lead)
            VStack(alignment: .leading, spacing: 0) {
                Text(title).font(tokens.font(.body)).foregroundStyle(tokens.textPrimary).lineLimit(1)
                    .frame(height: PanelMetrics.nameLine)
                if let detail {
                    Text(detail).font(tokens.font(.caption)).foregroundStyle(detailWarns ? tokens.statusWarningText : tokens.textMuted)
                        .lineLimit(2).frame(minHeight: PanelMetrics.metaLine, alignment: .topLeading)
                }
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            if hovering { ForEach(actions) { BandButton(symbol: $0.symbol, label: $0.label, inRow: true, action: $0.run) } }
            trailing
        }
        .padding(.horizontal, PanelMetrics.inset).padding(.vertical, detail == nil ? 0 : Space._1)
        .frame(minHeight: detail == nil ? Metric.row : Metric.rowTwoLine)
        if let action {
            PanelRow(action: action) { _ in content }.onHover { hovering = $0 }
        } else {
            content
                .background(hovering ? tokens.stateHover : .clear, in: RoundedRectangle(cornerRadius: Radius.row, style: .continuous))
                .onHover { hovering = $0 }
        }
    }
}

extension TwoLineRow where Trailing == EmptyView {
    init(
        title: String, detail: String? = nil, detailWarns: Bool = false, action: (() -> Void)? = nil, actions: [RowAction] = [],
        @ViewBuilder lead: () -> Lead
    ) {
        self.init(title: title, detail: detail, detailWarns: detailWarns, action: action, actions: actions, lead: lead, trailing: { EmptyView() })
    }
}
