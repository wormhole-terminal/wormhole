// Two things a row or a head wears for a while: the highlight of a row that just moved to another group (600 ms, fading
// out; `MOVE_FLASH`) and the focus ring while its context menu is open (`row/mod.rs`, `group_head.rs`: the same
// `focus_ring` as a keyboard stop, so it is the shared `keyboardFocusRing`, hidden in a window behind, in Increase
// Contrast's width). A SwiftUI `.contextMenu` reports no state, so the ring follows AppKit's menu tracking while the
// pointer is over the row.

import AppKit
import DesignSystem
import SwiftUI

struct RowFlash: View {
    let shape: RoundedRectangle
    @State private var faded = false
    @Environment(\.tokens) private var tokens

    /// How long the row that moved stays highlighted, fading out.
    static let duration = 0.6

    var body: some View {
        shape.fill(tokens.stateHighlight).opacity(faded ? 0 : 1).allowsHitTesting(false)
            .onAppear { withAnimation(.linear(duration: Self.duration)) { faded = true } }
    }
}

struct OpenMenuRing: ViewModifier {
    let hovering: Bool
    @State private var open = false

    func body(content: Content) -> some View {
        content
            .keyboardFocusRing(open, radius: Radius.row)
            .onReceive(NotificationCenter.default.publisher(for: NSMenu.didBeginTrackingNotification)) { _ in if hovering { open = true } }
            .onReceive(NotificationCenter.default.publisher(for: NSMenu.didEndTrackingNotification)) { _ in open = false }
    }
}

extension View {
    /// The focus ring while this view's context menu is open; `hovering`: the pointer is over it.
    func ringWhileMenuOpen(hovering: Bool) -> some View { modifier(OpenMenuRing(hovering: hovering)) }
}
