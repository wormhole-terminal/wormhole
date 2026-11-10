// Every animation the chrome plays, named once. Timings stay between 150 and 350 ms and the springs are
// SwiftUI's own (`.snappy`, `.smooth`), so they retarget mid-flight when a change arrives before the last
// one has finished. Under Reduce Motion each one becomes a short fade and nothing moves or scales.

import SwiftUI

public enum Motion {
    /// Every animation's length in seconds, by name: the animations below are built from these, and the Theme
    /// Studio's gallery lists them.
    public static let timings: [(name: String, seconds: Double)] = [
        ("press", 0.15), ("selection", 0.25), ("layout", 0.30), ("fade", 0.15), ("breath", 0.9),
    ]
    private static func seconds(_ name: String) -> Double { timings.first { $0.name == name }?.seconds ?? 0.25 }

    /// Hover and press feedback: quick enough to feel attached to the pointer.
    public static let press = Animation.snappy(duration: seconds("press"))
    /// A selection moving, a count changing, a glyph swapping.
    public static let selection = Animation.snappy(duration: seconds("selection"))
    /// Layout that changes size or order: a group folding, a row arriving or leaving.
    public static let layout = Animation.smooth(duration: seconds("layout"))
    /// What replaces movement when Reduce Motion is on.
    public static let fade = Animation.easeOut(duration: seconds("fade"))
    /// One breath of a status glyph; it repeats for as long as the state lasts.
    public static let breath = Animation.easeInOut(duration: seconds("breath"))
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
