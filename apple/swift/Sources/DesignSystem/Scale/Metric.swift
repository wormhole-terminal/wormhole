// Sizes that are not spacing: the GPUI scale's heights, glyphs and hit areas (`style/scale.rs`).

import SwiftUI

public enum Metric {
    /// A form control and an icon button's face (`CONTROL_H`).
    public static let control: CGFloat = 28
    /// A control inside a panel: a tab, a chip (`CONTROL_H_SM`).
    public static let controlSmall: CGFloat = 22
    /// What stands in a title row beside the 24 pt icon buttons (`TITLE_CONTROL_H`).
    public static let titleControl: CGFloat = 24
    /// A one-line list row (`ROW_H`).
    public static let row: CGFloat = 28
    /// A two-line list row: four above, a 20 name line, a 16 meta line, four below (`ROW_TWO_LINE_H`).
    public static let rowTwoLine: CGFloat = 44
    /// A folder band in the sidebar, a section band in the right panel (`BAND_H`).
    public static let band: CGFloat = 32
    /// The three title rows across the top (`TITLE_ROW_H`).
    public static let titleRow: CGFloat = 38
    /// The status bar's height (`STATUS_BAR_H`).
    public static let statusBar: CGFloat = 28
    /// The inset from a column's edge to its content, shared by header, rows and search (`EDGE_INSET`).
    public static let edgeInset: CGFloat = 12
    /// The lead column of a row: a status glyph, a chevron (`ITEM_LEAD_W`).
    public static let lead: CGFloat = 16
    /// How far each level of a tree steps in (`INDENT`).
    public static let indent: CGFloat = 24
    public static let glyph: CGFloat = 14
    public static let titleGlyph: CGFloat = 16
    /// The least size of anything clickable (`HIT_MIN`).
    public static let hitMin: CGFloat = 28
    public static let badgeHeight: CGFloat = 16
    public static let keyCapHeight: CGFloat = 20
    public static let keyCapMinWidth: CGFloat = 28
    public static let statusDot: CGFloat = 8
    public static let statusHalo: CGFloat = 3
    public static let ring: CGFloat = 1.5
    public static let chevron: CGFloat = 9
    /// The status tile beside a two-line row, and the small ones (`TILE`, `TILE_MINI`, `TILE_SMALL`).
    public static let tile: CGFloat = 32
    public static let tileMini: CGFloat = 20
    public static let tileSmall: CGFloat = 16
    /// The quick panel and command palette (`QUICK_PANEL_W`, `QP_SEARCH_H`, `QP_ROW_H`).
    public static let paletteWidth: CGFloat = 600
    public static let paletteSearchHeight: CGFloat = 48
    public static let paletteRowHeight: CGFloat = 36
}
