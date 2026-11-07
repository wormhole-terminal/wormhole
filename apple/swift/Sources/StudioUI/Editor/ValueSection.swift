// The value of the selected token: for a colour the picker, the value and its origin, then the expression field;
// for a metric the stepper and slider. Reset drops the theme's own value, back to what its rule works out.

import DesignSystem
import StudioModel
import SwiftUI

struct ValueSection: View {
    @Bindable var document: StudioDocument
    let token: String
    @Environment(\.tokens) private var tokens

    var body: some View {
        EditorSection {
            HStack {
                SectionLabel(StudioWords.sectionValue)
                if document.file.own(token, in: document.shown) != nil {
                    Button { document.reset(token) } label: {
                        Label(StudioWords.reset, systemImage: "arrow.counterclockwise").font(TextRole.caption.font)
                    }
                    .buttonStyle(.plain).foregroundStyle(tokens.textMuted).help(StudioWords.reset)
                }
            }
            if token.hasPrefix("metrics.") {
                MetricEditor(document: document, token: token)
            } else {
                ColorPickerRow(document: document, token: token)
                ExpressionField(document: document, token: token)
            }
        }
    }
}
