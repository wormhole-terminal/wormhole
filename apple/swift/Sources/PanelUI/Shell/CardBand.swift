// The head of a card (`style::section_card`): 32 tall, the chevron in the lead column when the section can
// fold, the title (13 / 600), then what the caller puts before the count, the count, and the +. Hovering it
// takes `state.hover`; clicking the title folds. A section with nothing to fold has no chevron and no click.

import DesignSystem
import SwiftUI

struct CardBand<Trailing: View>: View {
    let title: String
    /// `nil`: the section has nothing to fold (no chevron, no click).
    var folded: Bool?
    var count: String?
    /// What VoiceOver hears: the title with its count in words.
    var spoken: String?
    var onToggle: () -> Void = {}
    var add: (label: String, action: () -> Void)?
    @ViewBuilder var trailing: Trailing
    @State private var hovering = false
    @Environment(\.tokens) private var tokens

    var body: some View {
        HStack(spacing: PanelMetrics.gap) {
            toggle
            HStack(spacing: Space._1) {
                trailing
                if let count { BandCount(text: count) }
                if let add { BandButton(symbol: "plus", label: add.label, action: add.action) }
            }
        }
        .padding(.horizontal, PanelMetrics.inset)
        .frame(height: Metric.band)
        .background(hovering ? tokens.stateHover : .clear, in: RoundedRectangle(cornerRadius: Radius.row, style: .continuous))
        .onHover { hovering = $0 }
    }

    private var toggle: some View {
        Button(action: onToggle) {
            HStack(spacing: PanelMetrics.gap) {
                if let folded { Disclosure(collapsed: folded) }
                Text(title).font(TextRole.label.font).foregroundStyle(tokens.textPrimary).lineLimit(1)
                Spacer(minLength: 0)
            }
            .frame(maxHeight: .infinity)
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .allowsHitTesting(folded != nil)
        .accessibilityLabel(spoken ?? title)
        .accessibilityValue(folded.map { $0 ? panelText("panel-spoken-folded", "folded") : panelText("panel-spoken-open", "open") } ?? "")
    }
}

extension CardBand where Trailing == EmptyView {
    init(
        title: String, folded: Bool? = nil, count: String? = nil, spoken: String? = nil,
        onToggle: @escaping () -> Void = {}, add: (label: String, action: () -> Void)? = nil
    ) {
        self.init(
            title: title, folded: folded, count: count, spoken: spoken, onToggle: onToggle, add: add,
            trailing: { EmptyView() })
    }
}
