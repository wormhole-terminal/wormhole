// Sizes that are not spacing: the GPUI scale's heights, glyphs and hit areas (`style/scale.rs`).

import SwiftUI

public enum Metric {
    /// A form control and an icon button's face (`CONTROL_H`).
    public static var control: CGFloat { ScaleValue.of("metric.control", 28) }
    /// A control inside a panel: a tab, a chip (`CONTROL_H_SM`).
    public static var controlSmall: CGFloat { ScaleValue.of("metric.controlSmall", 22) }
    /// What stands in a title row beside the 24 pt icon buttons (`TITLE_CONTROL_H`).
    public static var titleControl: CGFloat { ScaleValue.of("metric.titleControl", 24) }
    /// A one-line list row (`ROW_H`).
    public static var row: CGFloat { ScaleValue.of("metric.row", 28) }
    /// A two-line list row: four above, a 20 name line, a 16 meta line, four below (`ROW_TWO_LINE_H`).
    public static var rowTwoLine: CGFloat { ScaleValue.of("metric.rowTwoLine", 44) }
    /// A folder band in the sidebar, a section band in the right panel (`BAND_H`).
    public static var band: CGFloat { ScaleValue.of("metric.band", 32) }
    /// The three title rows across the top (`TITLE_ROW_H`).
    public static var titleRow: CGFloat { ScaleValue.of("metric.titleRow", 38) }
    /// The status bar's height (`STATUS_BAR_H`).
    public static var statusBar: CGFloat { ScaleValue.of("metric.statusBar", 28) }
    /// The inset from a column's edge to its content, shared by header, rows and search (`EDGE_INSET`).
    public static var edgeInset: CGFloat { ScaleValue.of("metric.edgeInset", 12) }
    /// The lead column of a row: a status glyph, a chevron (`ITEM_LEAD_W`).
    public static var lead: CGFloat { ScaleValue.of("metric.lead", 16) }
    /// How far each level of a tree steps in (`INDENT`).
    public static var indent: CGFloat { ScaleValue.of("metric.indent", 24) }
    public static var glyph: CGFloat { ScaleValue.of("metric.glyph", 14) }
    public static var titleGlyph: CGFloat { ScaleValue.of("metric.titleGlyph", 16) }
    /// The least size of anything clickable (`HIT_MIN`).
    public static var hitMin: CGFloat { ScaleValue.of("metric.hitMin", 28) }
    public static var badgeHeight: CGFloat { ScaleValue.of("metric.badgeHeight", 16) }
    public static var keyCapHeight: CGFloat { ScaleValue.of("metric.keyCapHeight", 20) }
    public static var keyCapMinWidth: CGFloat { ScaleValue.of("metric.keyCapMinWidth", 28) }
    public static var statusDot: CGFloat { ScaleValue.of("metric.statusDot", 8) }
    public static var statusHalo: CGFloat { ScaleValue.of("metric.statusHalo", 3) }
    public static var ring: CGFloat { ScaleValue.of("metric.ring", 1.5) }
    public static var chevron: CGFloat { ScaleValue.of("metric.chevron", 9) }
    /// The status tile beside a two-line row, and the small ones (`TILE`, `TILE_MINI`, `TILE_SMALL`).
    public static var tile: CGFloat { ScaleValue.of("metric.tile", 32) }
    public static var tileMini: CGFloat { ScaleValue.of("metric.tileMini", 20) }
    public static var tileSmall: CGFloat { ScaleValue.of("metric.tileSmall", 16) }
    /// The quick panel and command palette (`QUICK_PANEL_W`, `QP_SEARCH_H`, `QP_ROW_H`).
    public static var paletteWidth: CGFloat { ScaleValue.of("metric.paletteWidth", 600) }
    public static var paletteSearchHeight: CGFloat { ScaleValue.of("metric.paletteSearchHeight", 48) }
    public static var paletteRowHeight: CGFloat { ScaleValue.of("metric.paletteRowHeight", 36) }
    /// A result with a second line (`ROW_TWO_LINE_H`).
    public static var paletteRowTwoLines: CGFloat { ScaleValue.of("metric.paletteRowTwoLines", 44) }
}
