// The two lines beside a row's tile: the name (14, medium; semibold on the selected row and while output is unseen)
// with its pin and, at the end, the keycap under ⌘ or the row's details; and under it the second line. A compact or dense row has
// one line only, as in core (`second_line` has no text there).

import DesignSystem
import SwiftUI
import WorkspaceModel

struct RowLines: View {
    let row: SidebarRow
    let isSelected: Bool
    /// The pointer is over the row: a × takes the end of the first line, so a long name fades out before it.
    let hovering: Bool
    let rename: RowRename?
    @Environment(\.tokens) private var tokens
    @Environment(\.sidebarMetrics) private var metrics
    @Environment(\.sidebarLook) private var look
    @Environment(\.sidebarNow) private var now
    @Environment(\.showsHints) private var showsHints

    private var terminal: TerminalSummary { row.terminal }
    private var line: SecondLine { SidebarWording.secondLine(for: terminal, now: now, home: look.home) }

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            HStack(spacing: Space._2) {
                name
                if terminal.isPinned { PinMark(isSelected: isSelected).transition(.scale(scale: 0.4).combined(with: .opacity)) }
                Spacer(minLength: 0)
                trail
            }
            .frame(height: RowGeometry.nameLine)
            if metrics.density.twoLines {
                SecondLineView(line: line, isSelected: isSelected, rolls: look.statusMotion == .lineRoll)
            }
        }
        .animation(SidebarMotion.hint, value: showsHints)
        .animation(SidebarMotion.state, value: terminal.activity)
    }

    @ViewBuilder private var name: some View {
        if let rename {
            RenameField(name: terminal.title, rename: rename)
        } else {
            NameText(
                name: terminal.title, font: TextRole.body.font.weight(isSelected || terminal.unread > 0 ? .semibold : .medium),
                color: isSelected ? tokens.sidebarRowSelectedText : tokens.textPrimary, reserve: hovering ? RowGeometry.closeRoom : 0)
        }
    }

    @ViewBuilder private var trail: some View {
        if showsHints, let number = row.number {
            Keycap(text: "⌘\(number)").transition(.opacity)
        } else {
            RowDetails(terminal: terminal, marks: row.splitMarks, isSelected: isSelected).opacity(hovering ? 0 : 1)
        }
    }
}
