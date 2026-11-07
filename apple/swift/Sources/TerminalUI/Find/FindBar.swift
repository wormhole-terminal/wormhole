// The find bar (`style::find_bar`): floats over the terminal's top right corner. The query field with its
// count and toggles, then previous, next and close. A click on it must not reach the terminal under it.

import DesignSystem
import SwiftUI

public struct FindBar: View {
    let model: FindModel
    let onClose: () -> Void
    @Environment(\.tokens) private var tokens

    public init(model: FindModel, onClose: @escaping () -> Void) {
        self.model = model
        self.onClose = onClose
    }

    public var body: some View {
        HStack(spacing: Space._1) {
            FindField(model: model, onClose: onClose)
            IconButton(symbol: "chevron.up", label: LocalizedStringKey(L10n.findPrevious)) { model.step(-1) }
            IconButton(symbol: "chevron.down", label: LocalizedStringKey(L10n.findNext)) { model.step(1) }
            IconButton(symbol: "xmark", label: LocalizedStringKey(L10n.findClose), action: onClose)
        }
        .padding(.leading, Space._2).padding(.trailing, Space._1)
        .frame(width: FindMetrics.barWidth, height: FindMetrics.barHeight)
        .floatingSurface()
        .accessibilityElement(children: .contain)
        .accessibilityLabel(L10n.findPlaceholder)
        // Esc anywhere in the bar (a button focused) closes it, as in GPUI's focus ladder.
        .onExitCommand(perform: onClose)
    }
}
