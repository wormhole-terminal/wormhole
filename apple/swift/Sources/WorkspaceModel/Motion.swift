// The window's motion as named numbers, so no view picks its own. Plain values because this module
// knows no UI framework: the UI modules turn them into `Animation`s (`.snappy(duration:)`,
// `.smooth(duration:)`, `.spring(response:dampingFraction:)`) and swap them for `reducedFade` when Reduce
// Motion is on. Short, so an animation never makes a person wait and is interrupted by the next action.

import Foundation

public enum Motion {
    /// Hover, press and focus feedback.
    public static let feedback: TimeInterval = 0.15
    /// A row or section appearing, leaving or moving; selection sliding between rows; a group folding.
    public static let standard: TimeInterval = 0.25
    /// A column sliding (sidebar, panel): the largest thing that moves, so the longest.
    public static let slide: TimeInterval = 0.35

    /// Springs for movement that should feel physical: no bounce on layout, a little on a press release.
    public static let layoutBounce: Double = 0
    public static let pressBounce: Double = 0.2

    /// What replaces any movement under Reduce Motion: the change still shows, as a quick cross-fade.
    public static let reducedFade: TimeInterval = 0.12

    /// How long ⌘ is held alone before the ⌘1–9 keycaps show (core's `HOLD_DELAY`), so ⌘C never flashes them.
    public static let keycapHoldDelay: TimeInterval = 0.25
}
