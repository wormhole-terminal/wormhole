// Every duration, spring and scale of the overlays, in one place (the lead swaps in DesignSystem's shared
// `Motion` here). All are short and springs are interruptible: a new target retargets from the current value.

import SwiftUI

struct PaletteMotion {
    /// With Reduce Motion on, nothing moves: the card only fades, and quickly.
    var reduceMotion: Bool

    /// Card in and out: a quick scale from 0.97 and a fade.
    var present: Animation { reduceMotion ? .easeOut(duration: 0.12) : .spring(duration: 0.25, bounce: 0.18) }
    var dismiss: Animation { reduceMotion ? .easeIn(duration: 0.1) : .spring(duration: 0.18, bounce: 0) }
    var backdrop: Animation { .easeOut(duration: reduceMotion ? 0.12 : 0.18) }
    var cardScale: CGFloat { reduceMotion ? 1 : 0.97 }

    /// The selection highlight sliding to another row.
    var selection: Animation? { reduceMotion ? nil : .spring(duration: 0.22, bounce: 0.05) }
    /// Rows reordering, entering and leaving while the query changes, and the list growing or shrinking.
    var results: Animation? { reduceMotion ? nil : .snappy(duration: 0.2) }
    /// Hover fills.
    var hover: Animation { .easeOut(duration: 0.1) }
}
