// A small icon button in a pane's header: a 22 pt face with a 14 pt glyph, a hover wash, a tooltip and a
// VoiceOver name. `DesignSystem`'s `IconButton` is the 28 pt one for title rows; headers are tighter.

import DesignSystem
import SwiftUI

struct PaneButton: View {
    let symbol: String
    let label: String
    /// The key that does the same, said in the tooltip ("Zoom pane (⇧⌘↩)").
    var key: String?
    let action: () -> Void
    @State private var hovering = false
    @Environment(\.tokens) private var tokens

    var body: some View {
        Button(action: action) {
            // Zoomed by the keys or by this button, the old corners shrink away as the new ones grow in.
            IconSwap(key: symbol) { Image(systemName: $0).font(.system(size: PaneMetrics.glyph, weight: .regular)) }
                .frame(width: PaneMetrics.button, height: PaneMetrics.button)
                .foregroundStyle(hovering ? tokens.textPrimary : tokens.textMuted)
                .background(hovering ? tokens.bandHover : .clear, in: RoundedRectangle(cornerRadius: tokens.metrics.radiusSm))
        }
        .buttonStyle(.plain)
        .onHover { hovering = $0 }
        .help(key.map { "\(label) (\($0))" } ?? label)
        .accessibilityLabel(label)
    }
}
