// How an SF Symbol could answer in the app's own language of icon motion (`IconMotion`): the symbol's own drawing changes
// and ends on its usual look, or on its other state for an icon that shows one, and it never shakes, bounces or pulses as
// a whole. A motion is a function of its progress (`SymbolMotionView`), so a still can show its middle. The timings and
// curves are the GPUI app's (`style/icon_motion/*`, `view::motion::ease`), as `IconMotion` has them.

import SwiftUI

/// One motion an SF Symbol could answer a click with, and what it needs to play.
public enum SymbolMotion: Equatable, Sendable {
    /// The side a fill grows from.
    public enum Edge: Equatable, Sendable { case leading, trailing, top, bottom }

    // MARK: A click plays one of these once, from progress 0 to 1, and the symbol ends as it began.
    /// The + a quarter round (`IconMotion.quarter`).
    case quarter
    /// The arrows once round (`IconMotion.spin`).
    case spin(clockwise: Bool)
    /// The front sheet fills and lets go as a check takes over, which gives the copy back (`IconMotion.copied`). The symbol is
    /// `doc.on.doc`, the alternate `checkmark`.
    case copied
    /// The half of a split the new pane takes grows from the divider outward and lets go (`IconMotion.newPane`), drawn with
    /// the symbol's own `.fill` variant, the alternate.
    case paneFill(IconMotion.Half)
    /// The alternate grows over the symbol from `edge` and lets go, as a pane's fill does.
    case pulse(Edge)
    /// The alternate grows from the centre as a disc and lets go.
    case disc
    /// The symbol drawn from `edge`, as a stroke is written.
    case written(Edge)
    /// The two lines of `note.text` written one after the other (`TabGlyph`'s notes).
    case lines
    /// The circle of `clock.arrow.circlepath` drawn round.
    case around
    /// The gauge's needle swept up from far back, a little past its place, and settled (`TabGlyph`'s gauge).
    case needle
    /// The alternate (the filled folder) flashes full and drains (`TabGlyph`'s folder).
    case flash

    // MARK: A click turns one of these from one state to the other, and the next back, as the toggles follow their column.
    /// The strip of `sidebar.left` or `sidebar.right` fills with its column (`PanelGlyph`).
    case strip(Edge)
    /// The alternate grows over the symbol from `edge`, and drains back.
    case wipe(Edge)
    /// The alternate grows from the centre as a disc, and shrinks back.
    case grow
    /// The symbol shrinks away and the alternate grows into its place (`IconSwap`).
    case swap
    /// The symbol turned half way over, or a quarter (a menu's chevron, a disclosure).
    case turn(degrees: Double)

    /// Whether a click changes a state (the next click changes it back) and not only plays.
    public var isState: Bool {
        switch self {
        case .strip, .wipe, .grow, .swap, .turn: true
        default: false
        }
    }

    /// How long a motion that plays once takes, in seconds (`icon_motion/*`).
    public var seconds: Double {
        switch self {
        case .quarter: IconMotion.Seconds.plusTurn
        case .spin: IconMotion.Seconds.arrowsSpin
        case .copied: IconMotion.Seconds.copied
        case .paneFill, .pulse, .disc: IconMotion.Seconds.paneFill
        case .written: 0.3
        case .lines, .around: 0.48
        case .needle: 0.52
        case .flash: 0.42
        case .strip, .wipe, .grow: 0.25
        case .swap: IconMotion.Seconds.swap
        case .turn: IconMotion.Seconds.chevronTurn
        }
    }

    /// The animation of a state's change, `opening` being towards the other state: a column comes in decelerating over 250
    /// ms and goes out accelerating over 200 (`ColumnSlide`), a chevron turns over in 200 the same way.
    public func animation(opening: Bool) -> Animation {
        switch self {
        case .swap: .timingCurve(0.2, 0, 0, 1, duration: seconds)
        case .turn: opening ? .timingCurve(0.2, 0, 0, 1, duration: seconds) : .timingCurve(0.3, 0, 1, 1, duration: seconds)
        default: opening ? .timingCurve(0.2, 0, 0, 1, duration: 0.25) : .timingCurve(0.3, 0, 1, 1, duration: 0.2)
        }
    }
}
