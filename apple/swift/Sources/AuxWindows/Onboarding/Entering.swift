// A part of the page coming in: it rises into place and fades in as far as the animation has let it (`Entering`).
// Under Reduce Motion nothing rises: the page is simply there (the glow stands in for the run), but a part still
// fades if a run is in progress.

import SwiftUI

extension View {
    /// `progress` 0 to 1: opacity follows it, and the part rises `OnboardingMetrics.rise` points into place.
    func entering(_ progress: Float) -> some View { modifier(Entering(progress: CGFloat(progress))) }
}

private struct Entering: ViewModifier {
    let progress: CGFloat
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    func body(content: Content) -> some View {
        content
            .offset(y: reduceMotion ? 0 : OnboardingMetrics.rise * (1 - progress))
            .opacity(progress)
            // A part that is not in yet is not there for VoiceOver or the keyboard either.
            .allowsHitTesting(progress > 0)
            .accessibilityHidden(progress < 0.01)
    }
}
