// What floats over the terminal: the elevated surface in a strong outline with the card corners and the
// shadow of something above the window (`style::find_bar`: 12 down, 32 blur). The outline sits inside the box.

import DesignSystem
import SwiftUI

extension View {
    func floatingSurface() -> some View { modifier(FloatingSurface()) }
}

private struct FloatingSurface: ViewModifier {
    @Environment(\.tokens) private var tokens

    func body(content: Content) -> some View {
        let shape = RoundedRectangle(cornerRadius: tokens.metrics.radiusLg)
        content
            .background(tokens.surfaceElevated, in: shape)
            .overlay { shape.strokeBorder(tokens.borderStrong, lineWidth: tokens.metrics.borderWidth) }
            // SwiftUI's radius is half GPUI's blur radius.
            .shadow(color: tokens.shadow, radius: Self.blur / 2, y: Self.offset)
    }

    /// `SHADOW_BLUR`, `SHADOW_Y` in `style/find_bar.rs`.
    private static let blur: CGFloat = 32
    private static let offset: CGFloat = 12
}
