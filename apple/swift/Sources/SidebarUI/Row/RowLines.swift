// The two lines beside a row's tile: the name (14, medium; semibold on the selected row and while output is unseen)
// with its pin, lock and worktree marks and, at the end, the keycap under ⌘ or the row's details; and under it the second line.
// A compact or dense row has one line only, as in core (`second_line` has no text there). A name or a second line that is cut
// off says itself whole in a tooltip.

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
    @Environment(\.showsHints) private var showsHints
    @State private var nameCut = false
    @State private var detailCut = false

    private var terminal: TerminalSummary { row.terminal }

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            HStack(spacing: Space._2) {
                name
                if terminal.isPinned {
                    RowMark(icon: .pin, isSelected: isSelected).transition(.scale(scale: 0.4).combined(with: .opacity))
                }
                if terminal.isAgentHidden { RowMark(icon: .lock, isSelected: isSelected, tip: SidebarText.hiddenFromAgents) }
                if let main = terminal.worktreeOf {
                    RowMark(icon: .gitFork, isSelected: isSelected, tip: SidebarText.worktree(terminal.title, of: main, at: terminal.folder))
                }
                Spacer(minLength: 0)
                trail
            }
            .frame(height: RowGeometry.nameLine)
            if metrics.density.twoLines {
                SecondLineView(line: row.secondLine, isSelected: isSelected, rolls: look.statusMotion == .lineRoll, cut: $detailCut)
            }
        }
        .help(nameCut || detailCut ? SidebarText.cut(terminal.title, place: terminal.folder, detail: row.secondLine.text) : "")
        .animation(SidebarMotion.hint, value: showsHints)
        .animation(SidebarMotion.state, value: terminal.activity)
    }

    @ViewBuilder private var name: some View {
        if let rename {
            RenameField(name: terminal.title, rename: rename)
        } else {
            NameText(
                name: terminal.title, font: TextRole.body.font.weight(isSelected || terminal.unread ? .semibold : .medium),
                color: isSelected ? tokens.sidebarRowSelectedText : tokens.textPrimary, reserve: hovering ? RowGeometry.closeRoom : 0,
                cut: $nameCut)
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
