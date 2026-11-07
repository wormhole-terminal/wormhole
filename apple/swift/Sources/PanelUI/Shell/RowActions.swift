// What an item row offers while the pointer is over it or it is being edited: send to the terminal and delete
// (`style::item_action`, the same two in the same order for a step and a note). They take the room at the
// row's right end, so the label truncates before them, and leave none at rest.

import SwiftUI

struct RowActions: View {
    let send: () -> Void
    let delete: () -> Void

    var body: some View {
        HStack(spacing: 0) {
            BandButton(symbol: "arrow.up.right", label: "Send to terminal", inRow: true, action: send)
            BandButton(symbol: "xmark", label: "Delete", inRow: true, action: delete)
        }
        .padding(.leading, PanelMetrics.gap)
    }
}
