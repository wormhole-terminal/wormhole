// The heading over a group of families, as tall as a family row so the list stays uniform (`entry_row`).

import DesignSystem
import FontPickerModel
import SwiftUI

struct FontCaptionView: View {
    let caption: FontEntry.Caption

    var body: some View {
        SectionLabel(title)
            .padding(.horizontal, Space._2)
            .frame(height: FontPickerMetric.rowHeight, alignment: .bottom)
            .accessibilityAddTraits(.isHeader)
    }

    private var title: String {
        switch caption {
        case .recent: FontWords.recent
        case .source(let source): source.label
        }
    }
}
