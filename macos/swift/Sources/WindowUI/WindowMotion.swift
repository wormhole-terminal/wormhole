// Every duration, spring and transition the window uses, in one file so they can be swapped for the design
// system's `Motion` in one place. All are short (150-350 ms) and take Reduce Motion: with it on, nothing
// slides or scales, things only fade (or, for a column's width, change at once).

import SwiftUI

enum WindowMotion {
    /// A column opening or closing and the title row moving with it. A spring, so a second toggle
    /// mid-flight carries on from where the first got to instead of restarting.
    static func column(reduceMotion: Bool) -> Animation? {
        reduceMotion ? nil : .smooth(duration: 0.3)
    }

    /// Content appearing or going: the quick part of every change.
    static let fade = Animation.easeOut(duration: 0.15)

    /// The tab strip's indicator and the body that follows it.
    static func tab(reduceMotion: Bool) -> Animation {
        reduceMotion ? fade : .snappy(duration: 0.25)
    }

    /// A button or chip entering or leaving a row.
    static func appear(reduceMotion: Bool) -> AnyTransition {
        reduceMotion ? .opacity : .opacity.combined(with: .scale(scale: 0.85))
    }

    /// The cross-fade between two terminals, in seconds.
    static let terminalSwitch: TimeInterval = 0.15
}
