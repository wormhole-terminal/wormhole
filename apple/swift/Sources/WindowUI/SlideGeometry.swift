// The window's body as a function of how far in each side column is. It is an animatable view: while a column
// slides, SwiftUI interpolates the two numbers along the slide's own curve and builds the body again for every
// frame, so the clips, the terminal's carry and the title row's padding are all computed from one moving value
// (`PaneFrames`) and stay together exactly, with no kink where a padding changes its mind.

import SwiftUI

struct SlideGeometry<Content: View>: View, @preconcurrency Animatable {
    var sidebarShown: CGFloat
    var panelShown: CGFloat
    let content: (_ sidebarShown: CGFloat, _ panelShown: CGFloat) -> Content

    var animatableData: AnimatablePair<CGFloat, CGFloat> {
        get { AnimatablePair(sidebarShown, panelShown) }
        set { (sidebarShown, panelShown) = (newValue.first, newValue.second) }
    }

    var body: some View { content(sidebarShown, panelShown) }
}
