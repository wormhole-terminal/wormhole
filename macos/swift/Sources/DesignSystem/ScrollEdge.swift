// The edge a scrolling list draws under what is above it (`panel_list.rs` `scroll_edges`): a hairline in `border` while rows
// are scrolled up beneath the top, as a Mac window draws one, none at rest. The sidebar's list and the right panel's tabs
// wear it. No fade at the bottom, as in the GPUI app: over a translucent surface a gradient darkens what it covers, and the
// row cut off at the edge already says there is more. The Borders setting clears `border`, and with it the line.

import SwiftUI

extension View {
    /// The hairline along this scroll view's top while its content is scrolled; put it on the `ScrollView` itself.
    public func scrollEdge() -> some View { modifier(ScrollEdge()) }
}

private struct ScrollEdge: ViewModifier {
    @State private var scrolled = false
    @Environment(\.tokens) private var tokens

    func body(content: Content) -> some View {
        content
            // Half a point counts, as GPUI's `is_scrolled`: a rubber band's settle never reads as scrolled.
            .onScrollGeometryChange(for: Bool.self) { $0.contentOffset.y + $0.contentInsets.top > 0.5 } action: { _, now in scrolled = now }
            .overlay(alignment: .top) {
                if scrolled {
                    Rectangle().fill(tokens.border).frame(height: tokens.metrics.borderWidth)
                        .allowsHitTesting(false)
                        .accessibilityHidden(true)
                }
            }
    }
}
