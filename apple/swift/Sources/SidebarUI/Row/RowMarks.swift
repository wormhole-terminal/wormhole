// What sits after a terminal's name and at its first line's end (`row/face.rs`): the pin after the name; at the end the
// chain of a split whose other members are elsewhere, the open notes and tasks, and the files changed ("±N"). The
// first line's end gives way to the × under the pointer, so these hide there.

import DesignSystem
import SwiftUI
import WorkspaceModel

struct PinMark: View {
    let isSelected: Bool
    @Environment(\.tokens) private var tokens

    var body: some View {
        LucideView(icon: .pin, size: RowGeometry.pin)
            .foregroundStyle(isSelected ? tokens.sidebarRowSelectedMuted : tokens.textFaint)
            .accessibilityHidden(true)
    }
}

struct RowDetails: View {
    let terminal: TerminalSummary
    let marks: SplitMarks
    let isSelected: Bool
    @Environment(\.tokens) private var tokens
    @Environment(\.sidebarLook) private var look
    @Environment(\.rowChanges) private var changes

    private var quiet: Color { isSelected ? tokens.sidebarRowSelectedMuted : tokens.textMuted }
    /// The size of the notes icon (`xsmall`): the pin's, so the two marks read as a set.
    private var glyph: CGFloat { RowGeometry.pin }

    var body: some View {
        HStack(spacing: Space._2) {
            if marks.chain {
                LucideView(icon: .link, size: Metric.glyph).foregroundStyle(tokens.textMuted)
                    .help("In a split with terminals in other folders")
            }
            if look.showsNotes, terminal.noteCount > 0 {
                HStack(spacing: Space._1) {
                    LucideView(icon: .note, size: glyph)
                    Text("\(terminal.noteCount)")
                }
            }
            if look.showsChanges, let count = changes[terminal.id] { Text("±\(count)") }
        }
        .font(TextRole.caption.font.monospacedDigit())
        .foregroundStyle(quiet)
        .lineLimit(1)
        .fixedSize()
    }
}
