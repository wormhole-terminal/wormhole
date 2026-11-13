// The keyboard focus ring every shared component draws the same way (`style::focus_ring`): `metrics.focus_ring_width` in
// `border.focus`, the control's own corners, only for keyboard focus and only in the window that is in front (a window
// behind keeps no ring, as in the GPUI app). The system's ring is off where this one is used.

import SwiftUI

extension View {
    /// The ring round this control while `shown` (keyboard focus), drawn `outset` points outside its edge so rows packed
    /// edge to edge pass `0`.
    public func keyboardFocusRing(_ shown: Bool, radius: CGFloat, outset: CGFloat = 0) -> some View {
        modifier(KeyboardFocusRing(shown: shown, radius: radius, outset: outset))
    }
}

extension View {
    /// The soft ring of a flat field or well (`flat_ring`): the input's focus colour at half strength, a point wider than the
    /// system ring because a field has no outline to hide it in; solid under Increase Contrast. Round the control, outside it.
    public func flatFocusRing(_ shown: Bool, radius: CGFloat) -> some View { modifier(FlatFocusRing(shown: shown, radius: radius)) }
}

private struct FlatFocusRing: ViewModifier {
    let shown: Bool
    let radius: CGFloat
    @Environment(\.tokens) private var tokens
    @Environment(\.increaseContrast) private var increased

    func body(content: Content) -> some View {
        content.overlay {
            if shown {
                let width = tokens.metrics.focusRingWidth + 1
                RoundedRectangle(cornerRadius: radius + width, style: .continuous)
                    .strokeBorder(tokens.inputBorderFocus.opacity(increased ? 1 : 0.5), lineWidth: width)
                    .padding(-width)
                    .allowsHitTesting(false)
                    .accessibilityHidden(true)
            }
        }
    }
}

private struct KeyboardFocusRing: ViewModifier {
    let shown: Bool
    let radius: CGFloat
    let outset: CGFloat
    @Environment(\.tokens) private var tokens
    @Environment(\.windowFront) private var windowFront

    func body(content: Content) -> some View {
        content.overlay {
            if shown && windowFront {
                RoundedRectangle(cornerRadius: radius + outset, style: .continuous)
                    .strokeBorder(tokens.borderFocus, lineWidth: tokens.metrics.focusRingWidth)
                    .padding(-outset)
                    .allowsHitTesting(false)
                    .accessibilityHidden(true)
            }
        }
    }
}
