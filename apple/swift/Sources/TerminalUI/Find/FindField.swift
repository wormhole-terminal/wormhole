// The query field of the bar: a search glyph, the text, the count and the three toggles inside one control
// outline that turns to the focus colour while the field has the keyboard. ↩ is next, ⇧↩ previous, esc closes.

import DesignSystem
import SwiftUI

struct FindField: View {
    let model: FindModel
    let onClose: () -> Void
    @Environment(\.tokens) private var tokens
    @FocusState private var focused: Bool
    @State private var selection: TextSelection?

    var body: some View {
        let shape = RoundedRectangle(cornerRadius: tokens.metrics.radius)
        HStack(spacing: Space._2) {
            LucideView(icon: .search, size: Metric.glyph).foregroundStyle(tokens.textMuted).accessibilityHidden(true)
            TextField(L10n.findPlaceholder, text: Binding(get: { model.text }, set: model.setText), selection: $selection)
                .textFieldStyle(.plain)
                .font(.system(size: TextRole.body.size))
                .focused($focused)
                .padding(.leading, FindMetrics.queryInset)
                .onKeyPress(.return, phases: .down) { press in
                    model.step(press.modifiers.contains(.shift) ? -1 : 1)
                    return .handled
                }
                .onKeyPress(.escape) { onClose(); return .handled }
            Text(model.countText)
                .font(.system(size: TextRole.caption.size)).monospacedDigit()
                .foregroundStyle(model.countIsError ? tokens.statusDangerText : tokens.textMuted)
                .lineLimit(1).frame(maxWidth: FindMetrics.countMaxWidth, alignment: .trailing)
                .accessibilityLabel(model.countText)
            ForEach(FindOption.allCases, id: \.self) { FindToggle(option: $0, model: model) }
        }
        .padding(.leading, Space._2)
        .frame(height: Metric.control)
        .background(tokens.inputBackground, in: shape)
        .overlay { shape.strokeBorder(focused ? tokens.borderFocus : tokens.border, lineWidth: tokens.metrics.borderWidth) }
        .onChange(of: model.focusRequest) { focus() }
        .onAppear { focus() }
    }

    /// The keyboard goes to the field with its text selected, to type over.
    private func focus() {
        focused = true
        selection = TextSelection(range: model.text.startIndex..<model.text.endIndex)
    }
}
