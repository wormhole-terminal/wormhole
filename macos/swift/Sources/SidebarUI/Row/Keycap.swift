// A shortcut as a key cap: the ⌘N hint a row shows while ⌘ is held (`style::controls::keycap`). A flat control-
// coloured chip, at least 28 wide so ⌘1 and ⌘9 match; Increase Contrast gives it its strong outline back.

import DesignSystem
import SwiftUI

struct Keycap: View {
    let text: String
    @Environment(\.tokens) private var tokens
    /// Increase Contrast is the system's setting, which the window's theme store follows.
    @Environment(\.increaseContrast) private var increased

    var body: some View {
        let shape = RoundedRectangle(cornerRadius: tokens.metrics.radiusChip)
        Text(text)
            .font(tokens.font(.caption).weight(.medium).monospacedDigit())
            .foregroundStyle(tokens.textPrimary)
            .lineLimit(1)
            .padding(.horizontal, Space._1)
            .frame(minWidth: Metric.keyCapMinWidth, minHeight: Metric.keyCapHeight)
            .background(tokens.controlBackground, in: shape)
            .overlay { if increased { shape.strokeBorder(tokens.borderStrong, lineWidth: tokens.metrics.borderWidth) } }
            .fixedSize()
            .marked("keycap")
    }
}
