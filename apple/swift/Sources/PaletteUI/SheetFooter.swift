// The sheet's footer: with a filter, how many rows it found of how many and what it matches; without one, where
// else the shortcuts are. A hairline over it. (The GPUI footer also names the global show/hide hotkey; this app
// has none yet, so it does not say so.)

import DesignSystem
import PaletteModel
import SwiftUI

struct SheetFooter: View {
    let found: FilteredShortcuts
    let filtering: Bool
    @Environment(\.tokens) private var tokens

    var body: some View {
        HStack(spacing: Space._2) {
            if filtering {
                Text("\(found.shown) of \(found.total) · matches labels and keys (“\(Shortcuts.closeTerminal.glyphs)” finds Close terminal)")
            } else {
                Spacer(minLength: 0)
                Text("Also in the command palette")
            }
        }
        .font(.system(size: TextRole.caption.size))
        .foregroundStyle(tokens.textMuted)
        .lineLimit(1)
        .padding(.horizontal, OverlayMetrics.sheetMargin)
        .frame(height: OverlayMetrics.sheetFooterHeight)
        .overlay(alignment: .top) { Rule(color: tokens.border, width: tokens.metrics.borderWidth) }
    }
}
