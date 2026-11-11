// A card of the list (`style/rows/card.rs`): the card's fill and hairline edge in the card's corner, and a group's colour
// washed over it: evenly at `WASH_PEAK`, or, with "Fade group colours", from the peak at the card's left edge down to
// `WASH_FAR` at `WASH_SPAN` of its width. Without cards (the person turned them off) it draws nothing and only sets the
// lines in by the sidebar's margin.

import DesignSystem
import SwiftUI

struct CardSurface<Content: View>: View {
    let wash: Color?
    let content: Content
    @Environment(\.tokens) private var tokens
    @Environment(\.sidebarMetrics) private var metrics
    @Environment(\.sidebarLook) private var look
    @Environment(\.cardGlass) private var glass

    init(wash: Color? = nil, @ViewBuilder content: () -> Content) {
        self.wash = wash
        self.content = content()
    }

    var body: some View {
        if metrics.cards {
            let shape = RoundedRectangle(cornerRadius: metrics.cardRadius)
            content
                .padding(metrics.cardPad + tokens.metrics.borderWidth)
                .background {
                    if glass { shape.fill(.clear).glassEffect(.regular, in: shape) } else { shape.fill(tokens.card) }
                    if let wash { shape.fill(washStyle(wash)) }
                }
                .overlay { shape.strokeBorder(tokens.cardEdge, lineWidth: tokens.metrics.borderWidth) }
                .clipShape(shape)
        } else {
            content
        }
    }

    private func washStyle(_ colour: Color) -> AnyShapeStyle {
        guard look.fadesGroupColours else { return AnyShapeStyle(colour.opacity(GroupWash.peak)) }
        return AnyShapeStyle(LinearGradient(
            stops: [.init(color: colour.opacity(GroupWash.peak), location: 0), .init(color: colour.opacity(GroupWash.far), location: GroupWash.span)],
            startPoint: .leading, endPoint: .trailing))
    }
}
