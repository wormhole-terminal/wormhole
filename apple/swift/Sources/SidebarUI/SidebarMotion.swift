// The list's motion, named once. The GPUI app's own timings first (`style/motion/timing.rs`: 120 ms fast,
// 160 ms base, `EASE_OUT` and `EASE_IN_OUT` as cubic beziers); springs only where it has none (a badge arriving,
// a card changing its state). Reduce Motion turns each into the short fade through `DesignSystem.motion`.

import SwiftUI

enum SidebarMotion {
    /// The ⌘ keycaps fading in (`MOTION_FAST`, `EASE_OUT`).
    static let hint = Animation.timingCurve(0.2, 0, 0, 1, duration: 0.12)
    /// A chevron turning and a group folding or opening (`MOTION_BASE`, `EASE_IN_OUT`).
    static let fold = Animation.timingCurve(0.4, 0, 0.2, 1, duration: 0.16)
    /// A folded head's tiles and count arriving once the rows are gone (`SUMMARY_FADE`).
    static let summary = Animation.easeOut(duration: 0.12)
    /// A card taking or losing the selection's tint, and hover: quick and symmetrical, so it can be reversed mid-way.
    static let state = Animation.snappy(duration: 0.18)
    /// A badge growing in past its size and settling, as the GPUI app's status change does.
    static let badge = Animation.spring(duration: 0.42, bounce: 0.5)
    /// Rows arriving, leaving and changing order.
    static let list = Animation.smooth(duration: 0.30)
    /// A light circling a working tile's edge, a turn (`TILE_ORBIT`), and the amber edge's breath (`TILE_BREATHE`).
    static let orbitTurn = 1.6
    static let breathe = 2.4
    static let breatheLeast = 0.45
}
