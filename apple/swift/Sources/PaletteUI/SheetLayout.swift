// How wide the sheet is and how its sections fall into columns (`shortcut_sheet.rs`): as many columns as there is
// room for, up to three; a filter that leaves fewer sections leaves fewer columns, and the sheet shrinks to
// them, each keeping its full width.

import DesignSystem
import PaletteModel
import SwiftUI

struct SheetLayout: Equatable {
    var width: CGFloat
    var columns: Int
    var columnWidth: CGFloat
    /// The sections' rows, a column at a time.
    var runs: [Range<Int>]
    /// The tallest column with its padding: what the body needs to show everything.
    var bodyHeight: CGFloat
    var maxHeight: CGFloat

    private static let sectionGap = Space._4
    private static let bodyTop = Space._2

    init(window: CGSize, sectionRows: [Int], filtering: Bool) {
        let margin = OverlayMetrics.sheetMargin
        let gap = OverlayMetrics.columnGap
        let widest = min(OverlayMetrics.sheetWidth, window.width - 2 * Metric.edgeInset)
        let full = Self.columnsFitting(width: widest)
        let columnWidth = (widest - 2 * margin - CGFloat(full - 1) * gap) / CGFloat(full)
        let columns = filtering ? min(full, max(sectionRows.count, 1)) : full
        let across = 2 * margin + CGFloat(columns) * columnWidth + CGFloat(columns - 1) * gap
        self.columnWidth = columnWidth
        self.columns = columns
        width = columns == full ? widest : min(max(across, OverlayMetrics.sheetMinWidth), widest)
        // A section is its label and its rows, all one line high (a label is a row's height too).
        let heights = sectionRows.map { $0 + 1 }
        runs = SheetColumns.split(heights, into: columns)
        var tallest: CGFloat = 0
        for run in runs {
            let lines = heights[run].reduce(0, +)
            let gaps = CGFloat(max(run.count - 1, 0)) * Self.sectionGap
            tallest = max(tallest, CGFloat(lines) * Metric.row + gaps)
        }
        bodyHeight = Self.bodyTop + tallest + Space._4
        maxHeight = window.height - OverlayMetrics.sheetTop - OverlayMetrics.bottomMargin
    }

    /// As many columns as `width` has room for, up to three.
    private static func columnsFitting(width: CGFloat) -> Int {
        let inner = width - 2 * OverlayMetrics.sheetMargin
        for n in [3, 2] {
            let needed = CGFloat(n) * OverlayMetrics.columnMinWidth + CGFloat(n - 1) * OverlayMetrics.columnGap
            if inner >= needed { return n }
        }
        return 1
    }

    /// What the body may be tall: the card's room less its header and footer.
    var bodyRoom: CGFloat { maxHeight - OverlayMetrics.sheetHeaderHeight - OverlayMetrics.sheetFooterHeight }
}
