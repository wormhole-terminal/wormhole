// The row the keys are on (`focus/panel.rs` `panel_cursor`): it wears the focus ring while its list has the keyboard,
// and the column scrolls to it. `SelectableListSplit` sets the cursor in the environment; a row names itself by the id
// its tab's `walkRows` gave it.

import DesignSystem
import SwiftUI

extension EnvironmentValues {
    /// The id of the row the keys are on, nil while the list does not hold the keyboard.
    @Entry var walkCursor: String?
}

extension View {
    func walkRow(_ id: String) -> some View { modifier(WalkRing(id: id)).id(id) }
}

private struct WalkRing: ViewModifier {
    let id: String
    @Environment(\.walkCursor) private var cursor
    @Environment(\.tokens) private var tokens

    func body(content: Content) -> some View {
        content.overlay {
            if cursor == id {
                RoundedRectangle(cornerRadius: Radius.row, style: .continuous)
                    .strokeBorder(tokens.borderFocus, lineWidth: tokens.metrics.focusRingWidth).allowsHitTesting(false)
            }
        }
    }
}
