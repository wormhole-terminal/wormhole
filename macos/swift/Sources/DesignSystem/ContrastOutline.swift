// Under macOS Increase Contrast a fill alone is no edge: the shapes that are told apart from what is behind them by a tint
// get a line (`outlined_always` in the GPUI app's `style/`), in the token the component names. Otherwise nothing is drawn.

import SwiftUI

private struct ContrastOutline<Outline: InsettableShape>: ViewModifier {
    let shape: Outline
    let color: (ThemeTokens) -> Color
    @Environment(\.tokens) private var tokens
    @Environment(\.increaseContrast) private var increased

    func body(content: Content) -> some View {
        content.overlay {
            if increased { shape.strokeBorder(color(tokens), lineWidth: tokens.metrics.borderWidth) }
        }
    }
}

extension View {
    /// An outline round `shape` in the theme's `edge` token while Increase Contrast is on.
    public func contrastOutline<Outline: InsettableShape>(_ shape: Outline, _ edge: KeyPath<ThemeTokens, Color>) -> some View {
        modifier(ContrastOutline(shape: shape, color: { $0[keyPath: edge] }))
    }

    /// The same in a colour of the component's own (a status badge's word).
    public func contrastOutline<Outline: InsettableShape>(_ shape: Outline, color: Color) -> some View {
        modifier(ContrastOutline(shape: shape, color: { _ in color }))
    }
}
