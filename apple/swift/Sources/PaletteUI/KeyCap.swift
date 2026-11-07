// A shortcut drawn as a key cap (`style::keycap`): one flat `control.background` pill for the whole chord, as
// tall as a caption line with room and at least as wide as "⌘1", so ⌘1 and esc line up. Increase Contrast gives
// it its outline back.

import DesignSystem
import SwiftUI

struct KeyCap: View {
    let text: String
    @Environment(\.tokens) private var tokens

    var body: some View {
        let shape = RoundedRectangle(cornerRadius: tokens.metrics.radiusChip)
        Text(text)
            .font(.system(size: TextRole.caption.size, weight: .medium))
            .monospacedDigit()
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
            .accessibilityElement(children: .ignore)
            .accessibilityLabel(Text(Self.spoken(text)))
    }

    /// VoiceOver reads glyphs badly ("⌘" as "place of interest"), so say the words.
    static func spoken(_ text: String) -> String {
        let words: [Character: String] = [
            "⌃": "Control", "⌥": "Option", "⇧": "Shift", "⌘": "Command", "↑": "Up arrow", "↓": "Down arrow",
            "←": "Left arrow", "→": "Right arrow", "⌫": "Delete", "↩": "Return",
        ]
        if text == "esc" { return "Escape" }
        return text.map { words[$0] ?? String($0) }.joined(separator: " ")
    }
}

/// A key and what it does, in a footer ("↩ open", "esc close"): the cap, then the words in muted caption text.
struct KeyHintView: View {
    let keys: String
    let label: String
    @Environment(\.tokens) private var tokens

    var body: some View {
        HStack(spacing: Space._2) {
            KeyCap(text: keys)
            Text(label).font(.system(size: TextRole.caption.size)).foregroundStyle(tokens.textMuted)
        }
        .fixedSize()
    }
}
