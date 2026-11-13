// What sits after a terminal's name and at its first line's end (`row/face.rs`): the pin, the lock and the worktree's fork after the name; at the end the
// chain of a split whose other members are elsewhere, the open notes and tasks, and the files changed ("±N"). The
// first line's end gives way to the × under the pointer, so these hide there.

import DesignSystem
import SwiftUI
import WordsModel
import WorkspaceModel

/// A small icon after the name: the pin, the lock of a terminal hidden from agents, the fork of a worktree. The last two
/// say what they mean in a two-line tooltip (`row/face.rs`).
struct RowMark: View {
    let icon: Lucide
    let isSelected: Bool
    var tip: String?
    @Environment(\.tokens) private var tokens

    var body: some View {
        LucideView(icon: icon, size: RowGeometry.pin)
            .foregroundStyle(isSelected ? tokens.sidebarRowSelectedMuted : tokens.textFaint)
            .help(tip ?? "")
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
                    .help(Words.text("sidebar-mark-split-folders", fallback: "In a split with terminals in other folders"))
            }
            if look.showsNotes, terminal.noteCount > 0 {
                HStack(spacing: Space._1) {
                    LucideView(icon: .note, size: glyph)
                    Text("\(terminal.noteCount)")
                }
            }
            if look.showsChanges, let count = changes[terminal.id] { Text("±\(count)") }
        }
        .font(tokens.font(.caption).monospacedDigit())
        .foregroundStyle(quiet)
        .lineLimit(1)
        .fixedSize()
    }
}
