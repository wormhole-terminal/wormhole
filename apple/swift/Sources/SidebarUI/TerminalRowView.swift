// One terminal in the list: status dot, name and a second line, then what the row has to say on its right.

import DesignSystem
import SwiftUI
import WorkspaceModel

struct TerminalRowView: View {
    let row: SidebarRow
    let isSelected: Bool
    /// The namespace the selection highlight slides in, owned by the list.
    let selection: Namespace.ID
    let send: @MainActor (WorkspaceAction) -> Void

    private var terminal: TerminalSummary { row.terminal }

    var body: some View {
        Row(isSelected: isSelected, selection: selection, action: { send(.select(terminal.id)) }) { hovering in
            HStack(spacing: Space._2) {
                StatusDot(terminal.activity.dot)
                VStack(alignment: .leading, spacing: 0) {
                    HStack(spacing: Space._1) {
                        Text(terminal.title)
                            .textRole(.body)
                            .fontWeight(isSelected ? .semibold : .medium)
                            .lineLimit(1)
                        if terminal.isPinned {
                            Image(systemName: "pin.fill")
                                .font(TextRole.caption.font).imageScale(.small).foregroundStyle(Palette.textSecondary)
                                .motionTransition(.scale(scale: 0.4).combined(with: .opacity))
                        }
                    }
                    if let detail {
                        Text(detail).textRole(.caption).foregroundStyle(Palette.textSecondary).lineLimit(1)
                    }
                }
                Spacer(minLength: 0)
                trailing(hovering: hovering)
            }
            .frame(minHeight: Metric.rowTwoLine)
        }
        .motion(Motion.selection, value: terminal.isPinned)
        .contextMenu {
            Button(terminal.isPinned ? "Unpin" : "Pin") { send(.togglePin(terminal.id)) }
            Button("Close") { send(.close(terminal.id)) }
        }
        .accessibilityElement(children: .ignore)
        .accessibilityLabel(SidebarListing.spokenLabel(for: terminal))
        .accessibilityAddTraits(isSelected ? [.isButton, .isSelected] : .isButton)
        .accessibilityAction(named: terminal.isPinned ? "Unpin" : "Pin") { send(.togglePin(terminal.id)) }
        .accessibilityAction(named: "Close") { send(.close(terminal.id)) }
    }

    /// The ⌘N hint takes the unread badge's place while the pointer is over the row, as the GPUI app's does.
    @ViewBuilder private func trailing(hovering: Bool) -> some View {
        Group {
            if hovering, let number = row.number {
                Text("⌘\(number)").textRole(.caption).foregroundStyle(Palette.textSecondary)
            } else if terminal.unread > 0 {
                CountBadge(terminal.unread)
            }
        }
        .motionTransition(.scale(scale: 0.6).combined(with: .opacity))
        .motion(Motion.selection, value: hovering)
        .motion(Motion.selection, value: terminal.unread > 0)
    }

    private var detail: String? {
        let parts = [terminal.subtitle, terminal.branch].compactMap { $0 }.filter { !$0.isEmpty }
        return parts.isEmpty ? nil : parts.joined(separator: " · ")
    }
}
