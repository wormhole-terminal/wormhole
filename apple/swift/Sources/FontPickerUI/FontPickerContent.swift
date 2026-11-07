// The open picker: the list on the left (search, the Monospace | All switch for terminal and code, the families,
// the count) and on the right what the highlighted font looks like as terminal text and what it can draw.

import DesignSystem
import FontPickerModel
import SwiftUI

struct FontPickerContent: View {
    @Bindable var model: FontPickerModel
    let look: PreviewLook
    var autofocus = true
    @Environment(\.tokens) private var tokens

    var body: some View {
        HStack(spacing: 0) {
            VStack(spacing: 0) {
                FontSearchField(model: model, autofocus: autofocus)
                if model.slot.prefersMonospace { toolbar }
                FontListView(model: model)
                if !model.slot.prefersMonospace { statusLine.padding(Space._2).frame(maxWidth: .infinity, alignment: .leading) }
            }
            .frame(width: FontPickerMetric.listWidth)
            Rectangle().fill(tokens.divider).frame(width: Rule.divider)
            FontPreviewView(model: model, look: look).frame(width: FontPickerMetric.previewWidth)
        }
        .onChange(of: model.catalog) { model.catalogChanged() }
        .accessibilityElement(children: .contain)
        .accessibilityLabel(FontWords.pickerLabel)
    }

    private var toolbar: some View {
        HStack {
            Picker(FontWords.pickerLabel, selection: $model.showsAll) {
                Text(FontWords.monospace).tag(false)
                Text(FontWords.all).tag(true)
            }
            .pickerStyle(.segmented).labelsHidden().fixedSize()
            Spacer(minLength: Space._2)
            statusLine
        }
        .padding(.horizontal, Space._2).padding(.vertical, Space._1)
        .overlay(alignment: .bottom) { Rectangle().fill(tokens.divider).frame(height: Rule.divider) }
    }

    private var statusLine: some View {
        Text(model.status).font(TextRole.caption.font).foregroundStyle(tokens.textFaint).lineLimit(1)
            .contentTransition(.numericText())
            .motion(Motion.selection, value: model.status)
    }
}
