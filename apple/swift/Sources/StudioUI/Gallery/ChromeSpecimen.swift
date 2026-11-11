// The primitives of `DesignSystem` as the chrome draws them: icon buttons, count badges, every status glyph, a
// section label folded and open, and a row in each state (`gallery/chrome.rs`).
// text-file: sample content of the Theme Studio gallery; the GPUI gallery shows its specimens untranslated too (studio/gallery)

import DesignSystem
import SwiftUI

struct ChromeSpecimen: View {
    @Namespace private var selection
    @Environment(\.tokens) private var tokens

    var body: some View {
        VStack(alignment: .leading, spacing: Space._4) {
            HStack(spacing: Space._1) {
                ForEach(["plus", "sidebar.left", "magnifyingglass", "gearshape", "xmark"], id: \.self) { symbol in
                    IconButton(symbol: symbol, label: LocalizedStringKey(symbol)) {}
                }
                Spacer(minLength: 0)
                CountBadge(3)
                CountBadge(1_234)
            }
            HStack(spacing: Space._4) {
                ForEach([StatusKind.idle, .working, .attention, .done, .failed], id: \.self) { StatusDot($0) }
            }
            VStack(alignment: .leading, spacing: 0) {
                SectionLabel("Open section", isCollapsed: false)
                SectionLabel("Folded section", isCollapsed: true)
            }
            VStack(spacing: Space._1) {
                Row(isSelected: false, selection: selection, action: {}) { _ in rowLabel("A row") }
                Row(isSelected: true, selection: selection, action: {}) { _ in rowLabel("The selected row") }
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }

    private func rowLabel(_ text: String) -> some View {
        Text(text).font(TextRole.body.font).frame(height: Metric.row).frame(maxWidth: .infinity, alignment: .leading)
    }
}
