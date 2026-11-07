// The ground of a secondary window: opaque `surface.window` (a window of text to read is not a place for the
// blur), the text colour, and room under the traffic lights for the title row the full-size content view hides.

import DesignSystem
import SwiftUI

extension View {
    /// Fills the window with the theme's window surface and pads the top by the title rows.
    func auxPage(titleRow: Bool = true) -> some View { modifier(AuxPage(titleRow: titleRow)) }
}

private struct AuxPage: ViewModifier {
    let titleRow: Bool
    @Environment(\.tokens) private var tokens

    func body(content: Content) -> some View {
        content
            .padding(.top, titleRow ? Metric.titleRow : 0)
            .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
            .foregroundStyle(tokens.textPrimary)
            .background(tokens.surfaceWindow)
            .ignoresSafeArea()
    }
}

/// A hairline in the theme's border colour (`rule_*`): nothing for a theme that draws no lines.
struct Rule: View {
    @Environment(\.tokens) private var tokens

    var body: some View {
        Rectangle().fill(tokens.border).frame(height: max(tokens.metrics.borderWidth, 0))
            .accessibilityHidden(true)
    }
}
