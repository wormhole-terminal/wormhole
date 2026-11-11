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
            // The icons that answer a click or a change with their own drawing (`IconMotion`), at rest: the + and the refresh
            // and copy glyphs by their icon, the splits, the zoom button's swap and the right panel's tab glyphs.
            HStack(spacing: Space._4) {
                IconButton(lucide: .plus, label: "Add") {}
                IconButton(lucide: .refreshCw, label: "Refresh") {}
                IconButton(lucide: .copy, label: "Copy") {}
                LucideView(icon: .columns2, size: Metric.titleGlyph).iconMotion(.newPane(.right), run: 0)
                LucideView(icon: .rows2, size: Metric.titleGlyph).iconMotion(.newPane(.below), run: 0)
                IconSwap(key: false) { _ in LucideView(icon: .chevronDown, size: Metric.glyph) }
                LucideView(icon: .chevronDown, size: Metric.glyph).menuChevron(open: true)
                ForEach([Lucide.gitCompare, .commit, .fileText, .gauge, .folder], id: \.self) { TabGlyph(icon: $0, size: Metric.glyph, run: 0) }
            }
            .foregroundStyle(tokens.textMuted)
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
