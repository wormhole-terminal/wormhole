// One side column coming in or going out, the way the GPUI app plays it (`view::motion::pose::column`,
// Classic): in over 250 ms easing out, out over 200 ms easing in. A column that comes in lies over the
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

    private(set) var shown: CGFloat
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
    func set(open wanted: Bool, animated: Bool) {
        guard wanted != open else { return }
        open = wanted
        serial += 1
        guard animated else {
            sliding = false
            shown = wanted ? 1 : 0
            inFlow = wanted
            return
        }
        sliding = true
        let mine = serial
        // Going out leaves the layout at once; coming in joins it when it has landed.
        if !wanted { inFlow = false }
        withAnimation(wanted ? Self.comingIn : Self.goingOut, completionCriteria: .logicallyComplete) {
            shown = wanted ? 1 : 0
        } completion: { [weak self] in
            guard let self, serial == mine else { return }
            sliding = false
            inFlow = open
        }
    }
}
