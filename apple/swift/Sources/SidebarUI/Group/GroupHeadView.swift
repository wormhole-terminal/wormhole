// A group's head (`style/groups.rs`): its name (13, semibold) with the pin after it and, when the person shows group folders,
// the folder under it (after the name in Dense); a dot of the group's colour before the name where there is no card to wash;
// at the right a chevron that turns when the group folds, a + under the pointer, and, folded, its terminals' tiles, their
// count and a dot while one needs input or failed. A group with nothing in it says "No terminals". The whole head folds
// and opens it; the name opens the group's popover.

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

    private var isEmpty: Bool { section.terminalCount == 0 && head.isPinned }
    /// The core has `~`-shortened it; a long one is cut from the front, as a row's place is.
    private var folder: String? { look.showsGroupFolders ? head.folder : nil }
    private var twoLines: Bool { folder != nil && metrics.density != .dense }
    private var height: CGFloat { twoLines ? metrics.headTwoLineHeight : isEmpty ? metrics.emptyHeight : metrics.headHeight }

    var body: some View {
        Button { context.send(.toggleGroup(head.id)) } label: {
            HStack(spacing: Space._2) {
                if !metrics.cards, let mark {
                    Circle().fill(mark).frame(width: Metric.statusDot, height: Metric.statusDot)
                }
                label
                Spacer(minLength: 0)
                if head.isFolded { FoldedSummary(tiles: section.tiles, count: section.terminalCount, alert: section.alert).transition(.opacity) }
                if hovering && !isEmpty { HeadAddButton { context.newInGroup(head.id) } }
                if isEmpty {
                    Text(Words.text("sidebar-group-empty", fallback: "No terminals")).font(TextRole.caption.font).foregroundStyle(tokens.textFaint)
                    HeadAddButton { context.newInGroup(head.id) }
                } else {
                    HeadChevron(isFolded: head.isFolded)
                }
            }
            .padding(.horizontal, metrics.rowInset)
            .frame(maxWidth: .infinity, minHeight: height, alignment: .leading)
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .onHover { hovering = $0 }
        .overlay {
            if keyed == .group(head.id) {
                RoundedRectangle(cornerRadius: Radius.row).strokeBorder(tokens.borderFocus, lineWidth: tokens.metrics.focusRingWidth)
            }
        }
        .animation(SidebarMotion.summary, value: head.isFolded)
        .opacity(context.drag.lifts(.group(head.id)) ? DragLift.opacity : 1)
        .dragSource(.group(head.id), hooks: context.drag)
        .dropFrame(.head(head.id))
        .contextMenu { GroupMenu(section: section, head: head, context: context) }
        .accessibilityElement(children: .ignore)
        .accessibilityLabel(head.spoken)
        .accessibilityAddTraits(.isButton)
        .accessibilityHint(Words.text("sidebar-group-head-hint", fallback: "Folds or opens the group"))
    }

    @ViewBuilder private var label: some View {
        VStack(alignment: .leading, spacing: 0) {
            HStack(spacing: Space._1) {
                Text(head.name).font(TextRole.label.font).foregroundStyle(tokens.textPrimary).lineLimit(1)
                if head.isPinned { LucideView(icon: .pin, size: RowGeometry.pin).foregroundStyle(tokens.textFaint) }
                if let folder, !twoLines { folderText(folder) }
            }
            if let folder, twoLines { folderText(folder) }
        }
    }

    private func folderText(_ folder: String) -> some View {
        Text(folder).font(TextRole.caption.font).foregroundStyle(tokens.textMuted).lineLimit(1).truncationMode(.head)
    }
}
