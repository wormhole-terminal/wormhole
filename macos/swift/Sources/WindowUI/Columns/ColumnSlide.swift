// One side column coming in or going out, the way the GPUI app plays it (`view::motion::pose::column`) under the
// person's `motion.column` choice. Classic: in over 250 ms easing out, out over 200 ms easing in. Soft spring: both ways
// on the soft spring (170/26). Content after: what is in the column fades out (80 ms) before it goes and in (150 ms, from
// 100) after it has begun to come. A column that comes in lies over the
// terminal and joins the layout when it is in (the terminal is resized once, at the end, never per frame);
// one that goes leaves the layout at once and is covered up as it goes. Reduce Motion makes both cuts.
//
// `shown` is how much of the column is in (0...1); views turn it into a clip width and the terminal's carry,
// and SwiftUI's own interpolation of those follows the same curve, so everything moves together.

import Observation
import SwiftUI

@MainActor @Observable
final class ColumnSlide {
    /// CSS `cubic-bezier(0.2, 0, 0, 1)` arriving and `(0.3, 0, 1, 1)` leaving, from core's `ease.rs`.
    static let comingIn = Animation.timingCurve(0.2, 0, 0, 1, duration: 0.25)
    static let goingOut = Animation.timingCurve(0.3, 0, 1, 1, duration: 0.2)
    /// Core's `SpringSpec::SOFT`, and Content after's stages (`FADE_OUT`, `FADE_IN`, `FADE_IN_AFTER`), in seconds.
    static let soft = Animation.interpolatingSpring(stiffness: 170, damping: 26)
    static let fadeOut = 0.08
    static let fadeIn = 0.15
    static let fadeInAfter = 0.1

    /// The clip's curve for the `motion.column` choice `motion` (`classic`, `soft-spring`, `content-after`).
    static func animation(_ motion: String, opening: Bool) -> Animation {
        switch motion {
        case "soft-spring": soft
        case "content-after" where !opening: goingOut.delay(fadeOut)
        default: opening ? comingIn : goingOut
        }
    }

    private(set) var shown: CGFloat
    /// How much of what is in the column shows (Content after fades it; 1 at rest and under the other choices).
    private(set) var content: CGFloat = 1
    /// Whether the column takes its place in the layout: not while it slides in over the terminal.
    private(set) var inFlow: Bool
    /// Whether a slide is under way, which is when the terminal is carried or cut instead of laid out.
    private(set) var sliding = false
    private var open: Bool
    private var serial = 0

    init(open: Bool) {
        self.open = open
        shown = open ? 1 : 0
        inFlow = open
    }

    /// The column should now be `open`. Without `animated` (a window resize hiding the panel for room) or
    /// with Reduce Motion it is a cut.
    func set(open wanted: Bool, animated: Bool, motion: String = "classic") {
        guard wanted != open else { return }
        open = wanted
        serial += 1
        guard animated else {
            sliding = false
            shown = wanted ? 1 : 0
            content = 1
            inFlow = wanted
            return
        }
        sliding = true
        let mine = serial
        // Going out leaves the layout at once; coming in joins it when it has landed.
        if !wanted { inFlow = false }
        if motion == "content-after" {
            // Coming from out, the content starts from nothing; turned round part-way, it goes on from what shows.
            if wanted, shown == 0 { content = 0 }
            let fade = wanted ? Animation.easeOut(duration: Self.fadeIn).delay(Self.fadeInAfter) : Animation.linear(duration: Self.fadeOut)
            withAnimation(fade) { content = wanted ? 1 : 0 }
        } else {
            content = 1
        }
        withAnimation(Self.animation(motion, opening: wanted), completionCriteria: .logicallyComplete) {
            shown = wanted ? 1 : 0
        } completion: { [weak self] in
            guard let self, serial == mine else { return }
            sliding = false
            content = 1
            inFlow = open
        }
    }
}
