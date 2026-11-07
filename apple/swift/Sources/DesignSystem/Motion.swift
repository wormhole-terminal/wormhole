// Every animation the chrome plays, named once. Timings stay between 150 and 350 ms and the springs are
// SwiftUI's own (`.snappy`, `.smooth`), so they retarget mid-flight when a change arrives before the last
// one has finished. Under Reduce Motion each one becomes a short fade and nothing moves or scales.

import SwiftUI

public enum Motion {
    /// Hover and press feedback: quick enough to feel attached to the pointer.
    public static let press = Animation.snappy(duration: 0.15)
    /// A selection moving, a count changing, a glyph swapping.
    public static let selection = Animation.snappy(duration: 0.25)
    /// Layout that changes size or order: a group folding, a row arriving or leaving.
    public static let layout = Animation.smooth(duration: 0.30)
    /// What replaces movement when Reduce Motion is on.
    public static let fade = Animation.easeOut(duration: 0.15)
    /// One breath of a status glyph; it repeats for as long as the state lasts.
    public static let breath = Animation.easeInOut(duration: 0.9)
    /// How far a pressed control sinks.
    public static let pressScale: CGFloat = 0.98
}

private struct MotionModifier<V: Equatable>: ViewModifier {
    let animation: Animation
    let value: V
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    func body(content: Content) -> some View {
        content.animation(reduceMotion ? Motion.fade : animation, value: value)
    }
}

private struct TransitionModifier: ViewModifier {
    let transition: AnyTransition
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    func body(content: Content) -> some View {
        content.transition(reduceMotion ? .opacity : transition)
    }
}

private struct PressModifier: ViewModifier {
    let isPressed: Bool
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    func body(content: Content) -> some View {
        content
            .scaleEffect(isPressed && !reduceMotion ? Motion.pressScale : 1)
            .animation(reduceMotion ? Motion.fade : Motion.press, value: isPressed)
    }
}

extension View {
    /// `animation(_:value:)` that plays the short fade instead when Reduce Motion is on.
    public func motion<V: Equatable>(_ animation: Animation, value: V) -> some View {
        modifier(MotionModifier(animation: animation, value: value))
    }

    /// `transition(_:)` that is a plain fade when Reduce Motion is on.
    public func motionTransition(_ transition: AnyTransition) -> some View {
        modifier(TransitionModifier(transition: transition))
    }

    /// The small sink of a held-down control; Reduce Motion keeps the colour change and drops the scale.
    public func pressed(_ isPressed: Bool) -> some View { modifier(PressModifier(isPressed: isPressed)) }
}
