// A group's head (`style/groups.rs`): its name (13, semibold) with the pin after it and, when the person shows group folders,
// the folder under it (after the name in Dense); a dot of the group's colour before the name where there is no card to wash;
// at the right a chevron that turns when the group folds, a + under the pointer, and, folded, its terminals' tiles, their
// count and a dot while one needs input or failed. A group with nothing in it says "No terminals". The whole head folds
// and opens it; the name opens the group's popover (its folder shows as a tooltip) and turns into a field to rename it.
// To VoiceOver the head is a container named by the core's words (name, colour, folder, count, open or folded), with what a
// pointer can press as buttons in it, as GPUI's head is (`groups.rs`): the name ("Edit group …"), the chevron ("Fold group
// …") and the + ("New terminal in …"), which stays in the tree while VoiceOver runs since it shows only under the pointer.

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
    @Environment(\.accessibilityVoiceOverEnabled) private var voiceOver

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
                Circle().fill(mark).frame(width: Metric.statusDot, height: Metric.statusDot).accessibilityHidden(true)
            }
            label
            Spacer(minLength: 0)
            // The head's own words say the count and the alert; the tiles are for the eyes.
            if head.isFolded {
                FoldedSummary(tiles: section.tiles, count: section.terminalCount, alert: section.alert).transition(.opacity).accessibilityHidden(true)
            }
            if (hovering || voiceOver) && !isEmpty { HeadAddButton(group: head.name) { context.newInGroup(head.id) } }
            if isEmpty {
                Text(Words.text("sidebar-group-empty", fallback: "No terminals")).font(tokens.font(.caption)).foregroundStyle(tokens.textFaint)
                HeadAddButton(group: head.name) { context.newInGroup(head.id) }
            } else {
                HeadChevron(isFolded: head.isFolded, spoken: foldWords) { context.send(.toggleGroup(head.id)) }
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
        .keyboardFocusRing(keyed == .group(head.id), radius: Radius.row)
        .animation(summaryAnimation, value: head.isFolded)
        .opacity(context.drag.lifts(.group(head.id)) ? DragLift.opacity : 1)
        .dragSource(.group(head.id), hooks: context.drag)
        .dropFrame(.head(head.id))
        .nativeContextMenu { context.hooks.groupMenu(head.id) }
        .accessibilityElement(children: .contain)
        .accessibilityLabel(head.spoken)
        .accessibilityHint(Words.text("sidebar-group-head-hint", fallback: "Folds or opens the group"))
        .accessibilityAction { context.send(.toggleGroup(head.id)) }
        .accessibilityAction(named: Words.text("sidebar-group-edit-action", fallback: "Edit group")) { context.openPopover(head.id) }
        .accessibilityAction(named: Words.text("sidebar-rename-action", fallback: "Rename")) { context.rename(.group(head.id)) }
    }

    @ViewBuilder private var label: some View {
        VStack(alignment: .leading, spacing: 0) {
            HStack(spacing: Space._1) {
                name
                if head.isPinned { LucideView(icon: .pin, size: RowGeometry.pin).foregroundStyle(tokens.textFaint).accessibilityHidden(true) }
                if let folder, !twoLines { folderText(folder) }
            }
            if let folder, twoLines { folderText(folder) }
        }
    }

    /// The name opens the popover (a click elsewhere on the head folds); renamed in place it is a field, and whatever ends it
    /// (↩, ⎋ or a click elsewhere) gives the keyboard back to the terminal.
    @ViewBuilder private var name: some View {
        if context.renaming == .group(head.id) {
            RenameField(name: head.name, font: tokens.font(.label), rename: RowRename { typed in
                context.rename(nil)
                if let typed { context.send(.renameGroup(head.id, typed)) }
                context.hooks.focusTerminal()
            })
        } else {
            Text(head.name).font(tokens.font(.label)).foregroundStyle(tokens.textPrimary).lineLimit(1)
                .contentShape(Rectangle())
                .onTapGesture { context.openPopover(head.id) }
                .help(head.folder ?? "")
                .accessibilityLabel(Words.text("sidebar-group-edit-spoken", ["group": .text(head.name)], fallback: "Edit group \(head.name)"))
                .accessibilityAddTraits(.isButton)
                .accessibilityAction { context.openPopover(head.id) }
        }
    }

    /// The chevron's name, which says what a press does (`spoken::fold_label`).
    private var foldWords: String {
        head.isFolded
            ? Words.text("sidebar-group-unfold-spoken", ["group": .text(head.name)], fallback: "Unfold group \(head.name)")
            : Words.text("sidebar-group-fold-spoken", ["group": .text(head.name)], fallback: "Fold group \(head.name)")
    }

    private func folderText(_ folder: String) -> some View {
        // Said in the head's own words.
        Text(folder).font(tokens.font(.caption)).foregroundStyle(tokens.textMuted).lineLimit(1).truncationMode(.head)
            .accessibilityHidden(true)
    }
}
