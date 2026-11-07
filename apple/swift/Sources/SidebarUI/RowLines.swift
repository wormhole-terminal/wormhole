// The two lines beside a row's tile: the name (14, medium; semibold on the selected row and while output is unseen)
// with its pin and, under ⌘, its keycap; and under it a caption of what the terminal is doing or where it is.
// A compact row has one line only.

import DesignSystem
import SwiftUI
import WorkspaceModel

struct RowLines: View {
    let terminal: TerminalSummary
    let number: Int?
    let isSelected: Bool
    /// The pointer is over the row: a × takes the end of the first line, so a long name fades out before it.
    let hovering: Bool
    @Environment(\.tokens) private var tokens
    @Environment(\.sidebarMetrics) private var metrics
    @Environment(\.showsHints) private var showsHints

    private var line: RowLine { RowLine.of(terminal, twoLines: metrics.density.twoLines) }
    private var muted: Color { isSelected ? tokens.sidebarRowSelectedMuted : tokens.textMuted }

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            HStack(spacing: Space._2) {
                NameText(
                    name: terminal.title, font: TextRole.body.font.weight(isSelected || terminal.unread > 0 ? .semibold : .medium),
                    color: isSelected ? tokens.sidebarRowSelectedText : tokens.textPrimary, reserve: hovering ? RowGeometry.closeRoom : 0)
                if terminal.isPinned {
                    Image(systemName: "pin").font(.system(size: RowGeometry.pin)).foregroundStyle(isSelected ? tokens.sidebarRowSelectedMuted : tokens.textFaint)
                        .transition(.scale(scale: 0.4).combined(with: .opacity))
                }
                Spacer(minLength: 0)
                if showsHints, let number {
                    Keycap(text: "⌘\(number)").transition(.opacity)
                }
            }
            .frame(height: RowGeometry.nameLine)
            if let text = line.text {
                HStack(spacing: Space._2) {
                    Text(text).font(TextRole.caption.font.weight(line.tone == .plain ? .medium : .regular)).foregroundStyle(color(line.tone))
                        .lineLimit(1).contentTransition(.interpolate)
                    Spacer(minLength: 0)
                    if let trail = line.trail {
                        Text(trail).font(TextRole.caption.font.monospacedDigit()).foregroundStyle(tokens.textFaint).lineLimit(1).fixedSize()
                    }
                }
                .frame(height: RowGeometry.metaLine)
            }
        }
        .animation(SidebarMotion.hint, value: showsHints)
        .animation(SidebarMotion.state, value: terminal.activity)
    }

    private func color(_ tone: RowTone) -> Color {
        switch tone {
        case .plain: muted
        case .attention: tokens.agentAttentionText
        case .done: tokens.agentDoneText
        case .failed: tokens.statusDangerText
        }
    }
}
