// An icon button in a band or on a row: the shared `IconButton` with the panel's own glyphs, a 24 pt face with a 16 pt
// glyph in a band (`in_band`), a 22 pt face with a 14 pt glyph on a row (`in_row`), still clickable across the 28 pt hit
// minimum. `selected` is a toggle that is on (the diff's wrap); `spoken` names the button where every row has the same
// tooltip (`IconButton::spoken`).

import DesignSystem
import SwiftUI

struct BandButton: View {
    let symbol: String
    let label: String
    var inRow = false
    var selected = false
    var spoken: String?
    let action: () -> Void

    var body: some View {
        let face = inRow ? PanelMetrics.rowButton : PanelMetrics.bandButton
        let button = IconButton(label: label, action: action) { _ in
            PanelIcon(symbol, size: inRow ? Metric.glyph : PanelMetrics.bandGlyph)
        }
        .answering(IconMotion.of(symbol: symbol))
        .selected(selected)
        .spoken(spoken)
        (inRow ? button.inRow() : button.inBand())
            // The hit box is wider than the face; the pull keeps the glyph, not the box, on the row's inset.
            .padding(.horizontal, -(Metric.hitMin - face) / 2)
    }
}
