// One frame of the intro: where the run has got to, and what is asked of the window around it.

import AuxModel
import Foundation

struct IntroFrame {
    /// Progress across the run, 0 to 1, and the seconds since it began.
    var t: Float
    var time: Float
    /// How opaque the traffic lights are while the overlay fades them away, or `nil` to leave them be.
    var lights: Float?
    /// After a skip: how the window was, and how far it has settled back to rest (0 to 1); the overlay fades away with it.
    var settling: (from: WindowMotion, settled: Float)?
}

/// What an overlay is asked to play.
struct IntroSetup {
    var style: IntroStyle
    var colors: IntroColors
    var start: IntroStart
    /// Whether the traffic lights fade away as the run begins (a replay over a showing window).
    var lights: Bool
    /// Whether Voyage's ship flies: not in the terminal welcome's Portal hand-over.
    var ship: Bool
    /// Over the menu bar and Dock too (the whole screen goes to space), or, in a test instance kept behind other
    /// apps, just above its window.
    var immersive: Bool
}
