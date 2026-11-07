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
    /// The count never takes more than a quarter of the bar (`FIND_BAR_W / 4`).
    static var countMaxWidth: CGFloat { barWidth / 4 }
}
