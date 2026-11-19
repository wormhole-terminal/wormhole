// A small icon button in a pane's header or on a tab (`icon_button(...).in_row()`, `hit_area`): the shared icon button on a
// row's 22 pt face with a 14 pt glyph, still clickable across the 28 pt hit, `band.hover` under the pointer. Its tooltip names
// the key that does the same ("Zoom pane (⇧⌘↩)"); VoiceOver hears the words and the key as the hint.

import DesignSystem
import SwiftUI

struct PaneButton: View {
    let symbol: String
    let label: String
    /// The key that does the same, said in the tooltip ("Zoom pane (⇧⌘↩)").
    var key: String?
    let action: () -> Void

    var body: some View {
        IconButton(label: key.map { "\(label) (\($0))" } ?? label, action: action) { _ in
            // Zoomed by the keys or by this button, the old corners shrink away as the new ones grow in.
            IconSwap(key: symbol) { Image(systemName: $0).font(.system(size: Metric.glyph, weight: .regular)) }
        }
        .inRow()
    }
}
