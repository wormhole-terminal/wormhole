// A shortcut drawn as one small cap per modifier and key, like the menu bar's key equivalents but flat.

import DesignSystem
import PaletteModel
import SwiftUI

struct KeyCaps: View {
    let shortcut: Shortcut

    var body: some View {
        HStack(spacing: 3) {
            ForEach(Array(shortcut.parts.enumerated()), id: \.offset) { _, part in
                Text(part)
                    .font(.system(size: 11, weight: .medium, design: .rounded))
                    .foregroundStyle(Palette.textSecondary)
                    .frame(minWidth: Space._4 + Space._1, minHeight: Space._4 + Space._1)
                    .padding(.horizontal, part.count > 1 ? Space._1 : 0)
                    .background(
                        RoundedRectangle(cornerRadius: Radius.chip - 2, style: .continuous)
                            .fill(Palette.hover)
                    )
            }
        }
        .accessibilityElement(children: .ignore)
        .accessibilityLabel(Text(spoken))
    }

    /// VoiceOver reads glyphs badly ("⌘" as "place of interest"), so say the words.
    private var spoken: String {
        let words = [
            "⌃": "Control", "⌥": "Option", "⇧": "Shift", "⌘": "Command", "↑": "Up arrow", "↓": "Down arrow",
        ]
        return shortcut.parts.map { words[$0] ?? $0 }.joined(separator: " ")
    }
}
