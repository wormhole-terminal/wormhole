// The quick panel's sizes, as the GPUI app has them (`style/scale.rs`, `quick_panel/view.rs`): the search row,
// a result row, a label and a quiet note, and where the window sits on the screen.

import CoreGraphics
import DesignSystem

enum QuickPanelMetrics {
    /// `QP_SEARCH_H`, `QP_ROW_H`, `ROW_H`: the same heights the command palette uses.
    static let searchHeight = Metric.paletteSearchHeight
    static let rowHeight = Metric.paletteRowHeight
    static let labelHeight = Metric.row
    /// Room above the first row (`PANEL_LIST_TOP`).
    static let listTop: CGFloat = 4
    /// How far from the top of the screen the window sits, as a share of the screen's height (`QP_TOP`).
    static let topShare: CGFloat = 0.2
    /// The indent of the answer region under a waiting row: lead column, inset and a gap (`ITEM_LEAD_W + PANEL_INSET + 8`).
    static let answerIndent: CGFloat = Metric.lead + Metric.edgeInset + Space._2
}
