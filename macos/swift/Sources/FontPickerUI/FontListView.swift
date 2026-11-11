// The families, one row each in a list that shrinks to a short result instead of leaving a gap. The highlight is
// one shape that slides between rows (the design system's `Row`); the keys bring the highlighted row into view,
// the pointer does not.

import DesignSystem
import FontPickerModel
import SwiftUI

struct FontListView: View {
    let model: FontPickerModel
    @Namespace private var highlight
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    var body: some View {
        ScrollViewReader { proxy in
            ScrollView {
                LazyVStack(spacing: 0) {
                    ForEach(model.entries) { entry in
                        switch entry {
                        case .caption(let caption): FontCaptionView(caption: caption)
                        default: FontRowView(model: model, entry: entry, highlight: highlight)
                        }
                    }
                    if let note = model.emptyNote { FontEmptyNote(text: note, hint: model.monospaceOnly) }
                }
                .padding(Space._1)
            }
            .frame(height: height)
            .onChange(of: model.scrollTarget) { _, id in
                guard let id else { return }
                withAnimation(reduceMotion ? nil : Motion.selection) { proxy.scrollTo(id, anchor: .center) }
            }
            .onAppear { if let id = model.scrollTarget { proxy.scrollTo(id, anchor: .center) } }
        }
        .accessibilityLabel(FontWords.pickerLabel)
    }

    private var height: CGFloat {
        min(max(CGFloat(model.entries.count), 1) * FontPickerMetric.rowHeight, FontPickerMetric.listHeight)
            + (model.emptyNote == nil ? 0 : FontPickerMetric.rowHeight)
    }
}

private struct FontEmptyNote: View {
    let text: String
    let hint: Bool
    @Environment(\.tokens) private var tokens

    var body: some View {
        VStack(alignment: .leading, spacing: Space._1) {
            Text(text).foregroundStyle(tokens.textMuted)
            if hint { Text(FontWords.showAllHint).foregroundStyle(tokens.textFaint) }
        }
        .textRole(.caption).padding(Space._2).frame(maxWidth: .infinity, alignment: .leading)
    }
}
