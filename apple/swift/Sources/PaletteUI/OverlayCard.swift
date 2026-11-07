// The card a palette is drawn on (`style::palette_surface`): the elevated surface in a strong outline with the
// card corners, and the shadow of something over the window. The outline sits inside the box, as GPUI's border does.

import DesignSystem
import SwiftUI

extension View {
    func overlayCard() -> some View { modifier(OverlayCard()) }
}

private struct OverlayCard: ViewModifier {
    @Environment(\.tokens) private var tokens

    func body(content: Content) -> some View {
        let shape = RoundedRectangle(cornerRadius: tokens.metrics.radiusLg)
        content
            .foregroundStyle(tokens.textPrimary)
            .background(tokens.surfaceElevated, in: shape)
            .clipShape(shape)
            .overlay { shape.strokeBorder(tokens.borderStrong, lineWidth: tokens.metrics.borderWidth) }
            .shadow(color: tokens.shadow, radius: OverlayMetrics.shadowBlur / 2, y: OverlayMetrics.shadowOffset)
    }
}
