// The find bar's own sizes, from `style/find_bar.rs`; spacing and corners are the shared scale and the theme's.

import CoreGraphics

enum FindMetrics {
    /// `FIND_BAR_W`, `FIND_BAR_H`.
    static let barWidth: CGFloat = 440
    static let barHeight: CGFloat = 40
    /// A match's tick along the pane's right edge (`overlays.rs`: `TICK_W`, `TICK_H`, `TICK_INSET`).
    static let tickWidth: CGFloat = 6
    static let tickHeight: CGFloat = 2
    static let tickInset: CGFloat = 2
    /// The step buttons (`in_title_row`: `TITLE_GLYPH` in `BAND_BUTTON`) and the toggles (`in_row`: `GLYPH` in `ROW_BUTTON`);
    /// each takes `HIT_MIN` in the row whatever its face.
    static let stepGlyph: CGFloat = 16
    static let toggleGlyph: CGFloat = 14
    static let hit: CGFloat = 28
    /// The query's own inset inside the field (the GPUI input's padding), measured on GPUI's shot: 10 pt after the gap.
    static let queryInset: CGFloat = 10
    /// The count never takes more than a quarter of the bar (`FIND_BAR_W / 4`).
    static var countMaxWidth: CGFloat { barWidth / 4 }
}
