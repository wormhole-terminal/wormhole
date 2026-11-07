// The sheet's columns of sections, read down then across, in the card's scrolling body; or the line that says
// nothing matched.

import DesignSystem
import PaletteModel
import SwiftUI

struct SheetBody: View {
    let found: FilteredShortcuts
    let layout: SheetLayout
    @Environment(\.tokens) private var tokens

    var body: some View {
        if found.shown == 0 {
            Text("No shortcut matches")
                .font(.system(size: TextRole.body.size))
                .foregroundStyle(tokens.textMuted)
                .padding(.horizontal, OverlayMetrics.sheetMargin)
                .padding(.vertical, Space._4)
                .frame(maxWidth: .infinity, alignment: .leading)
        } else {
            ScrollView {
                HStack(alignment: .top, spacing: OverlayMetrics.columnGap) {
                    ForEach(Array(layout.runs.enumerated()), id: \.offset) { _, run in column(run) }
                }
                .padding(.horizontal, OverlayMetrics.sheetMargin)
                .padding(.top, Space._2)
                .padding(.bottom, Space._4)
            }
            .scrollBounceBehavior(.basedOnSize)
            .frame(height: min(layout.bodyHeight, layout.bodyRoom))
        }
    }

    private func column(_ run: Range<Int>) -> some View {
        VStack(alignment: .leading, spacing: Space._4) {
            ForEach(Array(found.sections[run].enumerated()), id: \.offset) { _, section in
                VStack(spacing: 0) {
                    GroupLabel(title: section.title, count: nil, inset: 0)
                    ForEach(section.rows, id: \.row.id) { ShortcutLine(row: $0.row, hit: $0.hit) }
                }
            }
        }
        .frame(width: layout.columnWidth)
    }
}
