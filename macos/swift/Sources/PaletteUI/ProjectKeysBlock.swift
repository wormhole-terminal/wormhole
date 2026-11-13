// The trusted project's keys under the sheet's sections (`shortcut_sheet.rs`): a folder icon and "This project · name" with a
// Trusted badge over a hairline, then its actions' keys in the sheet's columns.

import DesignSystem
import PaletteModel
import SwiftUI
import WordsModel

struct ProjectKeysBlock: View {
    let keys: ProjectKeys
    let layout: SheetLayout
    @Environment(\.tokens) private var tokens

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            HStack(spacing: Space._2) {
                Image(systemName: "folder").font(.system(size: Metric.glyph)).accessibilityHidden(true)
                Text(Words.text("palette-sheet-project", ["project": .text(keys.name)], fallback: "This project · \(keys.name)")).font(.system(size: tokens.size(.label), weight: .medium))
                Spacer(minLength: 0)
                StatusBadge(text: Words.text("settings-projects-trusted", fallback: "Trusted"), color: tokens.statusSuccessText)
            }
            .foregroundStyle(tokens.textMuted)
            .frame(height: Metric.paletteRowHeight)
            LazyVGrid(
                columns: Array(repeating: GridItem(.fixed(layout.columnWidth), spacing: OverlayMetrics.columnGap), count: layout.columns),
                alignment: .leading, spacing: 0
            ) {
                ForEach(keys.rows) { ShortcutLine(row: $0) }
            }
        }
        .padding(.bottom, Space._4)
        .overlay(alignment: .top) { Rule(color: tokens.border, width: tokens.metrics.borderWidth) }
    }
}
