// A group's head (`style/groups.rs`): its name (13, semibold) alone on the line and a chevron at the right in a 20 box that
// turns when the group folds. Folded, the head keeps its terminals' tiles, their count and a dot while one needs input
// or failed; a group with nothing in it says "No terminals". The whole head folds and opens it.

import DesignSystem
import SwiftUI
import WorkspaceModel

struct GroupHeadView: View {
    let section: SidebarSection
    /// The group's terminals, for the tiles it keeps while folded.
    let members: [TerminalSummary]
    let send: @MainActor (WorkspaceAction) -> Void
    @Environment(\.tokens) private var tokens
    @Environment(\.sidebarMetrics) private var metrics
    @Environment(\.keyedStop) private var keyed
    @State private var chevronHover = false

    private var isEmpty: Bool { section.terminalCount == 0 }

    var body: some View {
        Button { send(.toggleGroup(section.id)) } label: {
            HStack(spacing: Space._2) {
                Text(section.title ?? "").font(TextRole.label.font).foregroundStyle(tokens.textPrimary).lineLimit(1)
                Spacer(minLength: 0)
                if section.isCollapsed { summary.transition(.opacity) }
                if isEmpty {
                    Text("No terminals").font(TextRole.caption.font).foregroundStyle(tokens.textFaint)
                } else {
                    chevron
                }
            }
            .padding(.horizontal, metrics.rowInset)
            .frame(maxWidth: .infinity, minHeight: isEmpty ? metrics.emptyHeight : metrics.headHeight, alignment: .leading)
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .overlay {
            if keyed == .group(section.id) {
                RoundedRectangle(cornerRadius: Radius.row).strokeBorder(tokens.borderFocus, lineWidth: tokens.metrics.focusRingWidth)
            }
        }
        .animation(SidebarMotion.summary, value: section.isCollapsed)
        .contextMenu { SidebarMenu.head(section, send: send) }
        .accessibilityElement(children: .ignore)
        .accessibilityLabel(SidebarListing.spokenLabel(for: section, position: 1, of: 1))
        .accessibilityAddTraits(.isButton)
        .accessibilityHint("Folds or opens the group")
    }

    private var chevron: some View {
        GlyphView(glyph: .chevron, size: RowGeometry.chevron)
            .foregroundStyle(tokens.textMuted)
            .rotationEffect(.degrees(section.isCollapsed ? 0 : 90))
            .animation(SidebarMotion.fold, value: section.isCollapsed)
            .frame(width: RowGeometry.chevronBox, height: RowGeometry.chevronBox)
            .background(chevronHover ? tokens.stateHover : .clear, in: RoundedRectangle(cornerRadius: tokens.metrics.radiusSm))
            .onHover { chevronHover = $0 }
            .animation(SidebarMotion.state, value: chevronHover)
    }

    /// The loudest first, as core sorts them; terminals alike keep the list's order.
    private var summary: some View {
        let tiles = members.sorted { TileState($0).urgency < TileState($1).urgency }
        let alert = tiles.first.map(TileState.init).flatMap { $0.isAlert ? $0 : nil }
        return HStack(spacing: RowGeometry.summaryGap) {
            ForEach(tiles) { terminal in
                StatusTile(state: TileState(terminal), size: metrics.foldedTile, phase: StatusTile.phase(of: terminal.id),
                           surface: tokens.sidebarBackground)
            }
            Text(CountBadge.label(section.terminalCount))
                .font(TextRole.caption.font.weight(.semibold).monospacedDigit()).foregroundStyle(tokens.textPrimary)
                .padding(.horizontal, Space._1)
                .frame(minWidth: Metric.badgeHeight, minHeight: Metric.badgeHeight)
                .background(tokens.border, in: RoundedRectangle(cornerRadius: tokens.metrics.radiusChip))
            if let alert {
                Circle().fill(alert == .needsYou ? tokens.agentAttention : tokens.statusDanger)
                    .frame(width: RowGeometry.alertDot, height: RowGeometry.alertDot)
            }
        }
    }
}
