// The ring the keyboard's place wears (`border.focus` at `metrics.focus_ring_width`), drawn only while it is there:
// the window decides when (`WindowFocus.ring`: the last input was a key, the window is in front).

import DesignSystem
import SwiftUI

extension View {
    func focusRing(_ on: Bool, radius: CGFloat) -> some View { modifier(FocusRing(on: on, radius: radius)) }
}

private struct FocusRing: ViewModifier {
    let on: Bool
    let radius: CGFloat

    func body(content: Content) -> some View {
        content.keyboardFocusRing(on, radius: radius)
    }
}
