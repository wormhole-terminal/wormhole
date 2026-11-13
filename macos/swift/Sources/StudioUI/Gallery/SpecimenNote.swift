// A part of a specimen with a quiet line beside it saying what it is ("← a 22 pt × in its 28 pt hit area"), as the GPUI
// gallery's notes stand beside its parts.

import DesignSystem
import SwiftUI

struct Noted<Content: View>: View {
    let note: String
    @ViewBuilder let content: Content
    @Environment(\.tokens) private var tokens

    init(_ note: String, @ViewBuilder content: () -> Content) {
        self.note = note
        self.content = content()
    }

    var body: some View {
        HStack(spacing: Space._3) {
            content
            Text(note).font(TextRole.caption.font).foregroundStyle(tokens.textMuted)
        }
    }
}
