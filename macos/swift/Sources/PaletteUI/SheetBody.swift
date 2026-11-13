// The sheet's columns of sections, read down then across, in the card's scrolling body; or the line that says
// nothing matched.

import DesignSystem
import PaletteModel
import SwiftUI
import WordsModel

struct SheetBody: View {
    let found: FilteredShortcuts
    let project: ProjectKeys?
    let layout: SheetLayout
    @Environment(\.tokens) private var tokens

    var body: some View {
        if found.shown == 0 {
            Text(Words.text("palette-sheet-none", fallback: "No shortcut matches"))
                .font(.system(size: tokens.size(.body)))
                .foregroundStyle(tokens.textMuted)
                .padding(.horizontal, OverlayMetrics.sheetMargin)
                .padding(.vertical, Space._4)
                .frame(maxWidth: .infinity, alignment: .leading)
        } else {
            ScrollView {
                VStack(spacing: 0) {
                    HStack(alignment: .top, spacing: OverlayMetrics.columnGap) {
                        ForEach(Array(layout.runs.enumerated()), id: \.offset) { _, run in column(run) }
                    }
                    .padding(.top, Space._2)
                    .padding(.bottom, Space._4)
                    if let project, !project.rows.isEmpty { ProjectKeysBlock(keys: project, layout: layout) }
                }
                .padding(.horizontal, OverlayMetrics.sheetMargin)
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
                    ForEach(section.rows) { ShortcutLine(row: $0) }
                }
            }
        }
        .frame(width: layout.columnWidth)
    }
}
