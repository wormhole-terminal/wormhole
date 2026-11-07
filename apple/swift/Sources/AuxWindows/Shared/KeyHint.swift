// A shortcut as a key cap and what it does ("⏎ continue", "esc skip"): one flat pill for the chord and the words
// in muted caption text (`style::keycap`). Increase Contrast gives the cap its outline back.

import DesignSystem
import SwiftUI

struct KeyCap: View {
    let text: String
    @Environment(\.tokens) private var tokens

    var body: some View {
        let shape = RoundedRectangle(cornerRadius: tokens.metrics.radiusChip)
        Text(text)
            .font(.system(size: TextRole.caption.size, weight: .medium))
            .foregroundStyle(tokens.textPrimary)
            .lineLimit(1)
            .fixedSize()
            .padding(.horizontal, Space._1)
            .frame(minWidth: Metric.keyCapMinWidth, minHeight: Metric.keyCapHeight, maxHeight: Metric.keyCapHeight)
            .background(tokens.controlBackground, in: shape)
            .overlay {
                if ThemeStore.shared.increaseContrast {
                    shape.strokeBorder(tokens.borderStrong, lineWidth: tokens.metrics.borderWidth)
                }
            }
            .accessibilityHidden(true)
    }
}

struct KeyHint: View {
    let keys: String
    let label: String
    @Environment(\.tokens) private var tokens

    var body: some View {
        HStack(spacing: Space._2) {
            KeyCap(text: keys)
            Text(label).font(TextRole.caption.font).foregroundStyle(tokens.textMuted)
        }
        .accessibilityElement(children: .ignore)
        .accessibilityLabel(Text("\(label)"))
        .fixedSize()
    }
}
