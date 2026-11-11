// The card a floating panel is drawn on (`style::palette_surface`): the elevated surface in a strong outline with
// the card corners, and the shadow of something over the window.

import DesignSystem
import SwiftUI

extension View {
    func panelCard() -> some View { modifier(PanelCard()) }
}

private struct PanelCard: ViewModifier {
    @Environment(\.tokens) private var tokens

    func body(content: Content) -> some View {
        let shape = RoundedRectangle(cornerRadius: tokens.metrics.radiusLg)
        content
            .foregroundStyle(tokens.textPrimary)
            .background(tokens.surfaceElevated, in: shape)
            .clipShape(shape)
            .overlay { shape.strokeBorder(tokens.borderStrong, lineWidth: tokens.metrics.borderWidth) }
    }
}

/// A hairline across a panel, between its search row, list and footer.
struct PanelRule: View {
    @Environment(\.tokens) private var tokens
    var body: some View { Rectangle().fill(tokens.border).frame(height: tokens.metrics.borderWidth) }
}
