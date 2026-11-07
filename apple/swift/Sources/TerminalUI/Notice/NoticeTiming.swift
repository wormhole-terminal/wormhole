// How the chip comes and goes: the numbers of `wormhole-core` `view::motion::notice`, which the design fixed.

import Foundation

enum NoticeTiming {
    /// The drop is a spring of stiffness 400, damping 30 (`SpringSpec::SNAP`), mass 1.
    static let dropStiffness = 400.0
    static let dropDamping = 30.0
    /// Where the chip rests: this far under the top edge of its terminal.
    static let restGap: CGFloat = 8
    /// Going back up, along ease-in (0.3, 0, 1, 1).
    static let up: TimeInterval = 0.18
    /// Reduce Motion: the chip fades in place.
    static let fadeIn: TimeInterval = 0.2
    static let fadeOut: TimeInterval = 0.15
    /// A new message while the chip shows: the old words crossfade into the new.
    static let swap: TimeInterval = 0.12
    /// How long a message stays; a message that replaces it starts this over.
    static let life: TimeInterval = 2
    /// The same message again within this long is not said twice.
    static let dedupe: TimeInterval = 2
    /// A click this soon after a chip that offers something appears is not taken for a choice.
    static let arm: TimeInterval = 0.5

    static func leaving(reduceMotion: Bool) -> TimeInterval { reduceMotion ? fadeOut : up }
}
