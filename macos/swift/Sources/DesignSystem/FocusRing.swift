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
