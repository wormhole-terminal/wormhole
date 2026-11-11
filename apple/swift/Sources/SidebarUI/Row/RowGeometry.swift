// The row's own proportions that `DesignSystem.Metric` does not hold: the two text lines of a regular row
// (`NAME_LINE` 1.25 rem and `META_LINE` 1 rem at the 16 pt rem), the pin's glyph, the close × and a head's chevron box
// (`sidebar/row/mod.rs`, `style/groups.rs`). Local until the design system takes them.

import SwiftUI

enum RowGeometry {
    static let nameLine: CGFloat = 20
    static let metaLine: CGFloat = 16
    static let pin: CGFloat = 12
    static let closeBox: CGFloat = 22
    static let closeTop: CGFloat = 3
    static let chevronBox: CGFloat = 20
    static let chevron: CGFloat = 12
    static let alertDot: CGFloat = 7
    static let summaryGap: CGFloat = 5
    /// How far a long name stops short of a × that shows under the pointer: the × glyph and a gap (`CLOSE_GLYPH_ROOM`).
    static let closeRoom: CGFloat = 18
    /// The group's colour at its fullest across a card (`WASH_PEAK`).
    static let washPeak: Double = 0.22
}
