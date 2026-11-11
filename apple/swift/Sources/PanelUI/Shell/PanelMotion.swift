// Every duration and curve of the panel, in one place, from the GPUI app's motion table (`style/motion`): hover
// has none, a selection fills in 120 ms on the ease-out curve (0.2, 0, 0, 1), a fold, an insert or a chevron turn
// takes 160 ms, a tab's content 140 ms, the segmented control's shape springs (stiffness 400, damping 30). Under
// Reduce Motion nothing moves: changes cross-fade in a hundred milliseconds or so.

import SwiftUI

struct PanelMotion {
    let reduced: Bool

    /// A selection filling in, or sliding between rows.
    var select: Animation { reduced ? fade : .timingCurve(0.2, 0, 0, 1, duration: 0.12) }
    /// A step ticking over: a little more life than the table has, since a tick is an action.
    var tick: Animation { reduced ? fade : .spring(duration: 0.3, bounce: 0.35) }
    /// Rows entering and leaving a list, a card folding or opening.
    var list: Animation { reduced ? fade : .timingCurve(0.4, 0, 0.2, 1, duration: 0.16) }
    var fold: Animation { list }
    /// The chevron's quarter turn.
    var turn: Animation { list }
    /// One tab's content replacing another's.
    var tab: Animation { reduced ? fade : .timingCurve(0.2, 0, 0, 1, duration: 0.14) }
    /// The segmented control's chosen shape (SNAP).
    var snap: Animation { reduced ? fade : .interpolatingSpring(stiffness: 400, damping: 30) }

    private var fade: Animation { .easeOut(duration: 0.12) }

    var row: AnyTransition { reduced ? .opacity : .opacity.combined(with: .offset(y: -6)) }
    /// A tab's content slides 10 pt as it fades in.
    var content: AnyTransition { reduced ? .opacity : .opacity.combined(with: .offset(x: 10)) }
    var hunk: AnyTransition { reduced ? .opacity : .opacity.combined(with: .offset(y: -6)) }
}

extension EnvironmentValues {
    var panelMotion: PanelMotion { PanelMotion(reduced: accessibilityReduceMotion) }
}
