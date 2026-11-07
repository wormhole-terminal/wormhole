// The frame every section of the token editor shares: 16 across, 12 down, a hairline under it (`section` in
// `studio/token_editor.rs`).

import DesignSystem
import SwiftUI

struct EditorSection<Content: View>: View {
    var spacing = Space._2
    @ViewBuilder let content: Content

    var body: some View {
        VStack(alignment: .leading, spacing: spacing) { content }
            .padding(.horizontal, Space._4).padding(.vertical, Space._3)
            .frame(maxWidth: .infinity, alignment: .leading)
            .hairline(.bottom)
    }
}

extension View {
    /// The code font at the caption's size: names, values and expressions.
    func codeFont() -> some View { font(TextRole.caption.font.monospaced()) }
}
