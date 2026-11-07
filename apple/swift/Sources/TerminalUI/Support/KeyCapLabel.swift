// A shortcut as a key cap (`style::keycap`): one flat `control.background` pill for the chord. The command
// palette module draws the same; both move to DesignSystem when the lead integrates (this one is a copy of
// `PaletteUI/KeyCap.swift`, kept private so a move is a delete here).

import DesignSystem
import SwiftUI

struct KeyCapLabel: View {
    let text: String
    @Environment(\.tokens) private var tokens

    var body: some View {
        Text(text)
            .font(.system(size: TextRole.caption.size, weight: .medium))
            .monospacedDigit()
            .foregroundStyle(tokens.textPrimary)
            .lineLimit(1)
            .fixedSize()
            .padding(.horizontal, Space._1)
            .frame(minWidth: Metric.keyCapMinWidth, minHeight: Metric.keyCapHeight, maxHeight: Metric.keyCapHeight)
            .background(tokens.controlBackground, in: RoundedRectangle(cornerRadius: tokens.metrics.radiusChip))
            .accessibilityHidden(true)
    }
}
