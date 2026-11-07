// One card of the panel: `card` fill and `card.edge` outline, 12 round, 4 of padding, 10 in from the panel's sides
// and 8 apart (`style::card_segment` at the regular density). Every tab is a stack of these.

import DesignSystem
import SwiftUI

struct PanelCard<Content: View>: View {
    @ViewBuilder let content: Content
    @Environment(\.tokens) private var tokens

    var body: some View {
        let shape = RoundedRectangle(cornerRadius: PanelMetrics.cardRadius, style: .continuous)
        VStack(spacing: 0) { content }
            .padding(PanelMetrics.cardPad)
            .background(tokens.card, in: shape)
            .overlay { shape.strokeBorder(tokens.cardEdge, lineWidth: tokens.metrics.borderWidth) }
            .padding(.horizontal, PanelMetrics.cardMargin)
            .padding(.bottom, PanelMetrics.cardGap)
    }
}

/// The scrolling column of cards, with the room above the first and the tail under the last.
struct PanelScroll<Content: View>: View {
    @ViewBuilder let content: Content

    var body: some View {
        ScrollView {
            VStack(spacing: 0) { content }
                .padding(.top, PanelMetrics.listTop)
                .padding(.bottom, PanelMetrics.tail - PanelMetrics.cardGap)
        }
        .scrollIndicators(.automatic)
    }
}

/// A line of quiet text where a list would be: loading, empty, not a repository (`style::empty_note`).
struct EmptyNote: View {
    let text: String
    @Environment(\.tokens) private var tokens

    var body: some View {
        Text(text)
            .font(TextRole.body.font)
            .foregroundStyle(tokens.textMuted)
            .padding(.horizontal, Space._3)
            .padding(.vertical, Space._2)
            .frame(maxWidth: .infinity, alignment: .leading)
    }
}
