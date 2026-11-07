// The token editor, on the right: the selected token's name and what it is for, its value (picker and
// expression, or a metric's stepper and slider), the rule that works it out, what draws with it, the contrast
// pairs it is part of, and the draft's pairs that fall short (`studio/token_editor.rs`).

import DesignSystem
import StudioModel
import SwiftUI

struct TokenEditorView: View {
    @Bindable var document: StudioDocument
    @Environment(\.tokens) private var tokens

    var body: some View {
        ScrollView {
            VStack(spacing: 0) {
                if let token = document.selected {
                    EditorHeader(document: document, token: token)
                    ValueSection(document: document, token: token)
                    RuleSection(document: document, token: token)
                    UsedBySection(document: document, token: token)
                    ContrastSection(document: document, token: token)
                } else {
                    Text(StudioWords.pickAToken).font(TextRole.body.font).foregroundStyle(tokens.textMuted)
                        .padding(Space._4).frame(maxWidth: .infinity, alignment: .leading)
                }
                LowContrastCard(document: document)
            }
        }
        .frame(width: StudioMetrics.editorWidth)
        .background(tokens.surfacePanel)
        .hairline(.leading)
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Token editor")
    }
}
