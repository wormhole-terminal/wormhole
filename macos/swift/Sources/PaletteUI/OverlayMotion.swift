// How the overlays come and go, as the design fixed it (core `view/motion/overlay.rs`, `style/motion`): the palette
// is there as the keys are pressed and fades out in 100 ms, easing in; the sheet fades in over 150 ms, easing out.
// Under Reduce Motion both are cuts. The extra movement (the selection sliding, the card growing) is native
// springs and stays off then too.

import SwiftUI

struct OverlayMotion {
    var reduceMotion: Bool

    /// The palette leaving (`EXIT`, `EASE_IN`).
    var exit: Animation? { reduceMotion ? nil : .timingCurve(0.3, 0, 1, 1, duration: 0.1) }
    /// The sheet arriving (`MOTION_POPOVER`, `EASE_OUT`).
    var entrance: Animation? { reduceMotion ? nil : .timingCurve(0.2, 0, 0, 1, duration: 0.15) }
    /// The highlight sliding to another row.
    var selection: Animation? { reduceMotion ? nil : .snappy(duration: 0.18) }
    /// The card growing or shrinking with its results.
    var resize: Animation? { reduceMotion ? nil : .snappy(duration: 0.22) }
}
