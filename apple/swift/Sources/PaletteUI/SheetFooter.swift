// The sheet's footer (`shortcut_sheet.rs`): with a filter, how many rows it found of how many and what it matches;
// without one, the key that shows or hides Wormhole from any app (or where to set it) and where else the shortcuts
// are. A hairline over it.

import DesignSystem
import PaletteModel
import SwiftUI

struct SheetFooter: View {
    let found: FilteredShortcuts
    let filtering: Bool
    /// The global hotkey as set, in Apple's notation.
    let hotkey: String?
    @Environment(\.tokens) private var tokens

    var body: some View {
        HStack(spacing: Space._2) {
            if filtering {
                Text("\(found.shown) of \(found.total) · matches labels and keys (“\(found.exampleKey)” finds Close terminal)")
            } else {
                Image(systemName: "globe").accessibilityHidden(true)
                if let hotkey {
                    Text("Show or hide Wormhole from any app")
                    KeyCap(text: hotkey)
                    Text("set in Settings › Hotkey").foregroundStyle(tokens.textFaint)
                } else {
                    Text("A key that shows or hides Wormhole from any app is set in Settings › Hotkey")
                }
                Spacer(minLength: 0)
                Text("Also in Help › Keyboard Shortcuts and the palette")
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
