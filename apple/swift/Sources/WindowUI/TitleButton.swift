// An icon button of the title rows: a 24 pt face with a 16 pt glyph, clickable across 28 pt, as the GPUI
// app's `icon_button(...).in_title_row()`. The glyph is muted until the pointer is over it, then primary on
// a hover wash. Native buttons would draw bezels that fight the flat rows, so this is the one place the look
// is ours. Hover has no animation, as in the GPUI app.

import DesignSystem
import SwiftUI

struct TitleButton<Glyph: View>: View {
    let label: String
    let help: String
    /// Which stop of the title row's ⇥ order this button is, where the keyboard can be on it.
    var stop: Int?
    /// Draws the glyph in the colour the button is giving it now.
    @ViewBuilder let glyph: (Color) -> Glyph
    var marked = false
    let action: @MainActor () -> Void
    @State private var hovering = false
    @Environment(\.chrome) private var colours
    @Environment(WindowFocus.self) private var focus

    var body: some View {
        Button(action: action) {
            glyph(hovering ? colours.text : colours.muted)
                .frame(width: Chrome.face, height: Chrome.face)
                .background(hovering ? colours.bandHover : .clear, in: RoundedRectangle(cornerRadius: colours.radius))
                .focusRing(stop.map { focus.ring(.titleRow, $0) } ?? false, radius: colours.radius)
                .overlay(alignment: .topTrailing) { if marked { MarkDot() } }
                .frame(width: Chrome.hit, height: Chrome.hit)
                .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .onHover { hovering = $0 }
        .help(help)
        .accessibilityLabel(label)
    }
}

/// The dot that says a wanted panel is waiting for room: the primary colour, ringed by the surface.
private struct MarkDot: View {
    @Environment(\.chrome) private var colours

    var body: some View {
        Circle().fill(colours.primary).frame(width: Chrome.markDot, height: Chrome.markDot)
            .padding(Chrome.markRing).background(colours.terminal, in: Circle())
            .padding(Chrome.markInset - Chrome.markRing - Chrome.markDot / 2)
    }
}
