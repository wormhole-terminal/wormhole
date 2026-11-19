// An icon button of the title rows (`icon_button(...).in_title_row()`): the shared `IconButton` on its 24 pt face, its
// glyph drawn by the caller in the colour the button gives it (the column toggles fill their strip), the keyboard ring
// where the title row's ⇥ order is on it, and the dot that says a wanted panel is waiting for room. `help` is the
// tooltip with the key; `label` is what VoiceOver hears.

import DesignSystem
import SwiftUI

struct TitleButton<Glyph: View>: View {
    let label: String
    let help: String
    /// Which stop of the title row's ⇥ order this button is, where the keyboard can be on it.
    var stop: Int?
    /// What the glyph does to answer a click (the + turns a quarter).
    var motion: IconMotion?
    /// Draws the glyph in the colour the button is giving it now.
    @ViewBuilder let glyph: (Color) -> Glyph
    var marked = false
    let action: @MainActor () -> Void
    @Environment(\.chrome) private var colours
    @Environment(WindowFocus.self) private var focus

    var body: some View {
        IconButton(label: help, action: { action() }, glyph: glyph)
            .inTitleRow()
            .answering(motion)
            .dot(marked, on: colours.terminal)
            .focusRing(stop.map { focus.ring(.titleRow, $0) } ?? false)
            .spoken(label)
    }
}
