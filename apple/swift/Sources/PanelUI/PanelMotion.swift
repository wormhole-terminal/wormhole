// Every duration and curve of the panel, in one place. Springs rather than fixed curves where an animation can
// be interrupted (a second click mid-slide retargets instead of snapping). With Reduce Motion on, nothing
// moves: changes cross-fade in a hundred milliseconds or so.

import SwiftUI

struct PanelMotion {
    let reduced: Bool

    /// Selection highlight sliding between rows.
    var select: Animation { reduced ? fade : .smooth(duration: 0.22) }
    /// A step ticking over.
    var tick: Animation { reduced ? fade : .spring(duration: 0.3, bounce: 0.35) }
    /// Rows entering and leaving a list.
    var list: Animation { reduced ? fade : .smooth(duration: 0.25) }
    /// A hunk folding, or the list above a diff making room.
    var fold: Animation { reduced ? fade : .smooth(duration: 0.25) }
    /// One tab's content replacing another's.
    var tab: Animation { reduced ? fade : .smooth(duration: 0.2) }

    private var fade: Animation { .easeOut(duration: 0.12) }

    var row: AnyTransition { reduced ? .opacity : .opacity.combined(with: .move(edge: .top)) }
    var content: AnyTransition { reduced ? .opacity : .opacity.combined(with: .scale(scale: 0.98)) }
    var hunk: AnyTransition { reduced ? .opacity : .opacity.combined(with: .offset(y: -6)) }
}

extension EnvironmentValues {
    var panelMotion: PanelMotion { PanelMotion(reduced: accessibilityReduceMotion) }
}
