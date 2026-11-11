// The sizes the palette and the shortcut sheet are laid out in, as the GPUI app has them
// (`command_palette/view.rs`, `shortcut_sheet.rs`, `style/palette.rs`). The ones every list shares (a row, the key
// cap, the lead column) are `DesignSystem.Metric`; these belong to the two overlays.

import CoreGraphics

enum OverlayMetrics {
    /// How far below the window's top the palette opens, and the most it grows to before its list scrolls.
    static let paletteTop: CGFloat = 72
    static let paletteMaxHeight: CGFloat = 560
    /// The least room left under the palette, and beside it in a narrow window.
    static let bottomMargin: CGFloat = 24
    static let sideMargin: CGFloat = 16
    /// Room under the last result.
    static let listBottom: CGFloat = 8
    /// The shadow of a card over the window (`FLOAT_SHADOW_Y`, `FLOAT_SHADOW_BLUR`).
    static let shadowOffset: CGFloat = 24
    static let shadowBlur: CGFloat = 64
    /// A segmented control's track reaches this far round its segments.
    static let segmentInset: CGFloat = 2
    /// A chip standing in a row's text line (the scope on the search field).
    static let chipHeight: CGFloat = 20

    // The shortcut sheet.
    static let sheetTop: CGFloat = 56
    static let sheetWidth: CGFloat = 1120
    static let sheetMinWidth: CGFloat = 520
    static let sheetHeaderHeight: CGFloat = 56
    static let sheetFooterHeight: CGFloat = 40
    static let sheetFilterWidth: CGFloat = 240
    static let sheetMargin: CGFloat = 24
    static let columnGap: CGFloat = 40
    static let columnMinWidth: CGFloat = 280
}
