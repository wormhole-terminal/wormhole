// A group's head (`style/groups.rs`): its name (13, semibold) with the pin after it and, when the person shows group folders,
// the folder under it (after the name in Dense); a dot of the group's colour before the name where there is no card to wash;
// at the right a chevron that turns when the group folds, a + under the pointer, and, folded, its terminals' tiles, their
// count and a dot while one needs input or failed. A group with nothing in it says "No terminals". The whole head folds
// and opens it; the name opens the group's popover (its folder shows as a tooltip) and turns into a field to rename it.

import DesignSystem
import SwiftUI
import WordsModel
import WorkspaceModel

struct GroupHeadView: View {
    let section: SidebarSection
    let head: GroupHead
    let mark: Color?
    let context: SidebarContext
    @State private var hovering = false
    @Environment(\.tokens) private var tokens
    @Environment(\.sidebarMetrics) private var metrics
    @Environment(\.sidebarLook) private var look
    @Environment(\.keyedStop) private var keyed
    @Environment(\.reduceMotion) private var reduceMotion

    private var isEmpty: Bool { section.terminalCount == 0 && head.isPinned }
    /// The core has `~`-shortened it; a long one is cut from the front, as a row's place is.
    private var folder: String? { look.showsGroupFolders ? head.folder : nil }
    private var twoLines: Bool { folder != nil && metrics.density != .dense }
    private var height: CGFloat { twoLines ? metrics.headTwoLineHeight : isEmpty ? metrics.emptyHeight : metrics.headHeight }

    /// The folded summary waits for the rows to be gone, as the core's `summary_opacity` does (Classic shows it at once).
    private var summaryAnimation: Animation {
        let wait = head.isFolded && !reduceMotion ? RowMotion.tables?.foldPlan(look.foldMotion, false, section.terminalCount).seconds : nil
        return wait.map { $0 > 0 ? SidebarMotion.summary.delay($0) : SidebarMotion.summary } ?? SidebarMotion.summary
    }

    var body: some View {
        HStack(spacing: Space._2) {
            if !metrics.cards, let mark {
                Circle().fill(mark).frame(width: Metric.statusDot, height: Metric.statusDot)
            }
            label
            Spacer(minLength: 0)
            if head.isFolded { FoldedSummary(tiles: section.tiles, count: section.terminalCount, alert: section.alert).transition(.opacity) }
            if hovering && !isEmpty { HeadAddButton(group: head.name) { context.newInGroup(head.id) } }
            if isEmpty {
                Text(Words.text("sidebar-group-empty", fallback: "No terminals")).font(TextRole.caption.font).foregroundStyle(tokens.textFaint)
                HeadAddButton(group: head.name) { context.newInGroup(head.id) }
            } else {
                HeadChevron(isFolded: head.isFolded)
            }
        }
        .padding(.horizontal, metrics.rowInset)
        .frame(maxWidth: .infinity, minHeight: height, alignment: .leading)
        .contentShape(Rectangle())
        .onTapGesture { context.send(.toggleGroup(head.id)) }
        .marked("sidebar.band")
        .onHover { hovering = $0 }
        .ringWhileMenuOpen(hovering: hovering)
        .onGeometryChange(for: CGRect.self) { $0.frame(in: .global) } action: { context.frames.heads[head.id] = $0 }
        .overlay {
            if keyed == .group(head.id) {
                RoundedRectangle(cornerRadius: Radius.row).strokeBorder(tokens.borderFocus, lineWidth: tokens.metrics.focusRingWidth)
            }
        }
        .animation(summaryAnimation, value: head.isFolded)
        .opacity(context.drag.lifts(.group(head.id)) ? DragLift.opacity : 1)
        .dragSource(.group(head.id), hooks: context.drag)
        .dropFrame(.head(head.id))
        .nativeContextMenu { context.hooks.groupMenu(head.id) }
        .accessibilityElement(children: .ignore)
        .accessibilityLabel(head.spoken)
        .accessibilityAddTraits(.isButton)
        .accessibilityHint(Words.text("sidebar-group-head-hint", fallback: "Folds or opens the group"))
        .accessibilityAction { context.send(.toggleGroup(head.id)) }
        .accessibilityAction(named: Words.text("sidebar-group-edit-action", fallback: "Edit group")) { context.openPopover(head.id) }
        .accessibilityAction(named: Words.text("sidebar-rename-action", fallback: "Rename")) { context.rename(.group(head.id)) }
    }

    @ViewBuilder private var label: some View {
        VStack(alignment: .leading, spacing: 0) {
            HStack(spacing: Space._1) {
                name
                if head.isPinned { LucideView(icon: .pin, size: RowGeometry.pin).foregroundStyle(tokens.textFaint) }
                if let folder, !twoLines { folderText(folder) }
            }
            if let folder, twoLines { folderText(folder) }
        }
    }

    /// The name opens the popover (a click elsewhere on the head folds); renamed in place it is a field, and whatever ends it
    /// (↩, ⎋ or a click elsewhere) gives the keyboard back to the terminal.
    @ViewBuilder private var name: some View {
        if context.renaming == .group(head.id) {
            RenameField(name: head.name, font: TextRole.label.font, rename: RowRename { typed in
                context.rename(nil)
                if let typed { context.send(.renameGroup(head.id, typed)) }
                context.hooks.focusTerminal()
            })
        } else {
            Text(head.name).font(TextRole.label.font).foregroundStyle(tokens.textPrimary).lineLimit(1)
                .contentShape(Rectangle())
                .onTapGesture { context.openPopover(head.id) }
                .help(head.folder ?? "")
        }
    }

    private func folderText(_ folder: String) -> some View {
        Text(folder).font(TextRole.caption.font).foregroundStyle(tokens.textMuted).lineLimit(1).truncationMode(.head)
    }
}
