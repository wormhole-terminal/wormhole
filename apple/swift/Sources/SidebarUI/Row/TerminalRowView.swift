// One terminal in the list (`sidebar/row/`): its tile, then the name over a second line. A terminal in no group is a
// card of its own (`solo`): the card takes the selection's tint and the hover, the row draws neither. In a group's card
// (or a split's) the row is a rounded stretch with its own. The pointer shows the × at the line's end; ⌘ held shows
// the keycap; a split's line runs behind the tile; a status change washes the row when the person chose that.

import DesignSystem
import SwiftUI
import WorkspaceModel

struct TerminalRowView: View {
    let row: SidebarRow
    let isSelected: Bool
    let solo: Bool
    let context: SidebarContext
    @State private var hovering = false
    @Environment(\.tokens) private var tokens
    @Environment(\.sidebarMetrics) private var metrics
    @Environment(\.sidebarLook) private var look
    @Environment(\.showsHints) private var showsHints
    @Environment(\.keyedStop) private var keyed
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    private var terminal: TerminalSummary { row.terminal }
    private var inset: CGFloat { metrics.rowInset + (solo ? metrics.cardPad : 0) }
    private var shape: RoundedRectangle { RoundedRectangle(cornerRadius: solo ? metrics.cardRadius : metrics.rowRadius) }

    var body: some View {
        ZStack(alignment: .topTrailing) {
            Button { context.send(.select(terminal.id)) } label: { content }
                .buttonStyle(.plain)
            if hovering && !showsHints && context.renaming != terminal.id {
                CloseButton(isSelected: isSelected) { context.send(.close(terminal.id)) }
                    .padding(.top, RowGeometry.closeTop).padding(.trailing, inset - (RowGeometry.closeBox - Metric.glyph) / 2)
                    .transition(.opacity)
            }
        }
        // The card's hairline takes room of its own, as the GPUI card's border does: a lone card is 48 + 2 high.
        .padding(solo ? tokens.metrics.borderWidth : 0)
        .background {
            shape.fill(fill)
            if let part = row.splitMarks.bracket { SplitLine(part: part, x: inset + metrics.tile / 2) }
        }
        .overlay { if solo { shape.strokeBorder(isSelected ? tokens.sidebarRowSelected : tokens.cardEdge, lineWidth: tokens.metrics.borderWidth) } }
        .overlay { StatusWash(state: tileState, count: reduceMotion ? 0 : context.shifts[terminal.id] ?? 0, enabled: look.statusMotion == .wash, shape: shape) }
        .overlay {
            if keyed == .row(terminal.id) {
                RoundedRectangle(cornerRadius: Radius.row).strokeBorder(tokens.borderFocus, lineWidth: tokens.metrics.focusRingWidth)
            }
        }
        .contentShape(shape)
        .onHover { hovering = $0 }
        .onAppear { hovering = look.hovered == terminal.id }
        .animation(SidebarMotion.state, value: hovering)
        .animation(SidebarMotion.state, value: isSelected)
        .opacity(context.drag.lifts(.terminal(terminal.id)) ? DragLift.opacity : 1)
        .dragSource(.terminal(terminal.id), hooks: context.drag)
        .dropFrame(.row(terminal.id))
        .onGeometryChange(for: CGRect.self) { $0.frame(in: .global) } action: { context.frames.frames[terminal.id] = $0 }
        .contextMenu { RowMenu(terminal: terminal, context: context) }
        .accessibilityElement(children: .ignore)
        .accessibilityLabel(row.spoken)
        .accessibilityAddTraits(isSelected ? [.isButton, .isSelected] : .isButton)
        .accessibilityAction { context.send(.select(terminal.id)) }
        .accessibilityAction(named: terminal.isPinned ? "Unpin" : "Pin") { context.send(.togglePin(terminal.id)) }
        .accessibilityAction(named: "Rename") { context.rename(terminal.id) }
        .accessibilityAction(named: "Close terminal") { context.send(.close(terminal.id)) }
    }

    /// The card's own fill when alone; the row's when it stands in a group's card (whose fill shows through).
    private var fill: Color {
        if isSelected { return tokens.sidebarRowSelected }
        if hovering { return tokens.stateHover }
        return solo ? tokens.card : .clear
    }

    private var content: some View {
        HStack(spacing: metrics.tileGap) {
            StatusTile(state: tileState, agent: terminal.agent, size: metrics.tile, phase: StatusTile.phase(of: terminal.id),
                       surface: isSelected ? tokens.sidebarRowSelected : tokens.sidebarBackground)
            RowLines(row: row, isSelected: isSelected, hovering: hovering, rename: rename)
        }
        .padding(.horizontal, inset)
        .padding(.vertical, metrics.density.twoLines ? Space._1 : 0)
        .frame(maxWidth: .infinity, minHeight: metrics.rowHeight, alignment: .leading)
        .contentShape(Rectangle())
    }

    /// With Status dot off every tile is the quiet one (`row/tile.rs` `tile_parts`).
    private var tileState: TileState { look.showsStatus ? TileState(terminal) : .shell }

    private var rename: RowRename? {
        context.renaming == terminal.id ? RowRename { name in
            context.rename(nil)
            if let name { context.send(.rename(terminal.id, name)) }
        } : nil
    }
}
