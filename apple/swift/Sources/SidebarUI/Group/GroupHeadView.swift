// A group's head (`style/groups.rs`): its name (13, semibold) with the pin after it and, when the person shows group folders,
// the folder under it (after the name in Dense); a dot of the group's colour before the name where there is no card to wash;
// at the right a chevron that turns when the group folds, a + under the pointer, and, folded, its terminals' tiles, their
// count and a dot while one needs input or failed. A group with nothing in it says "No terminals". The whole head folds
// and opens it; the name opens the group's popover when the window can edit groups.

import DesignSystem
import SwiftUI
import WorkspaceModel

struct GroupHeadView: View {
    let section: SidebarSection
    /// The group's terminals, for the tiles it keeps while folded.
    let members: [TerminalSummary]
    let mark: Color?
    let context: SidebarContext
    @State private var hovering = false
    @Environment(\.tokens) private var tokens
    @Environment(\.sidebarMetrics) private var metrics
    @Environment(\.sidebarLook) private var look
    @Environment(\.keyedStop) private var keyed

    private var group: TerminalGroup? { section.group }
    private var isEmpty: Bool { section.terminalCount == 0 && group?.isPinned == true }
    private var folder: String? {
        guard look.showsGroupFolders, let path = group?.folder else { return nil }
        // A quiet shell's second line is its folder, `~`-shortened and cut from the front: the group's line reads alike.
        return SidebarWording.secondLine(for: TerminalSummary(id: 0, title: "", folder: path), now: Date(), home: look.home).text
    }
    private var twoLines: Bool { folder != nil && metrics.density != .dense }
    private var height: CGFloat { twoLines ? metrics.headTwoLineHeight : isEmpty ? metrics.emptyHeight : metrics.headHeight }

    var body: some View {
        Button { context.send(.toggleGroup(section.id)) } label: {
            HStack(spacing: Space._2) {
                if !metrics.cards, let mark {
                    Circle().fill(mark).frame(width: Metric.statusDot, height: Metric.statusDot)
                }
                label
                Spacer(minLength: 0)
                if section.isCollapsed { FoldedSummary(members: members, count: section.terminalCount).transition(.opacity) }
                if hovering && !isEmpty, let group { HeadAddButton(group: group.id) { newTerminal(in: group.id) } }
                if isEmpty, let group {
                    Text("No terminals").font(TextRole.caption.font).foregroundStyle(tokens.textFaint)
                    HeadAddButton(group: group.id) { newTerminal(in: group.id) }
                } else {
                    HeadChevron(isFolded: section.isCollapsed)
                }
            }
            .padding(.horizontal, metrics.rowInset)
            .frame(maxWidth: .infinity, minHeight: height, alignment: .leading)
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .onHover { hovering = $0 }
        .overlay {
            if keyed == .group(section.id) {
                RoundedRectangle(cornerRadius: Radius.row).strokeBorder(tokens.borderFocus, lineWidth: tokens.metrics.focusRingWidth)
            }
        }
        .animation(SidebarMotion.summary, value: section.isCollapsed)
        .opacity(context.drag.lifts(.group(section.id)) ? DragLift.opacity : 1)
        .dragSource(.group(section.id), hooks: context.drag)
        .dropFrame(.head(section.id))
        .contextMenu { GroupMenu(section: section, context: context) }
        .accessibilityElement(children: .ignore)
        .accessibilityLabel(SidebarListing.spokenLabel(for: section, position: 1, of: 1))
        .accessibilityAddTraits(.isButton)
        .accessibilityHint("Folds or opens the group")
    }

    @ViewBuilder private var label: some View {
        VStack(alignment: .leading, spacing: 0) {
            HStack(spacing: Space._1) {
                Text(section.title ?? "").font(TextRole.label.font).foregroundStyle(tokens.textPrimary).lineLimit(1)
                if group?.isPinned == true { LucideView(icon: .pin, size: RowGeometry.pin).foregroundStyle(tokens.textFaint) }
                if let folder, !twoLines { folderText(folder) }
            }
            if let folder, twoLines { folderText(folder) }
        }
    }

    private func folderText(_ folder: String) -> some View {
        Text(folder).font(TextRole.caption.font).foregroundStyle(tokens.textMuted).lineLimit(1)
    }

    private func newTerminal(in group: String) {
        context.send(.newTerminal)
        context.send(.addToGroup(context.nextTerminalID, group))
    }
}
