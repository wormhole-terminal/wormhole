// One terminal in the list (`sidebar/row/`): its tile, then the name over a second line. A terminal in no group is a
// card of its own (`solo`): the card takes the selection's tint and the hover, the row draws neither. In a group's card
// (or a split's) the row is a rounded stretch with its own. The pointer shows the × at the line's end; ⌘ held shows
// the keycap; a split's line runs behind the tile; a status change washes the row when the person chose that.

import DesignSystem
import SwiftUI
import WordsModel
import WorkspaceModel

struct TerminalRowView: View {
    let row: SidebarRow
    let isSelected: Bool
    let solo: Bool
    let context: SidebarContext
    /// Drawn as the picture of the row being dragged: its card alone, with the shadow of something floating.
    var lifted = false
    @State private var hovering = false
    @Environment(\.tokens) private var tokens
    @Environment(\.sidebarMetrics) private var metrics
    @Environment(\.sidebarLook) private var look
    @Environment(\.showsHints) private var showsHints
    @Environment(\.keyedStop) private var keyed
    @Environment(\.reduceMotion) private var reduceMotion
    @Environment(\.cardGlass) private var glass

    private var terminal: TerminalSummary { row.terminal }
    private var inset: CGFloat { metrics.rowInset + (solo ? metrics.cardPad : 0) }
    private var shape: RoundedRectangle { RoundedRectangle(cornerRadius: solo ? metrics.cardRadius : metrics.rowRadius) }

    var body: some View {
        if lifted { liftedFace } else { interactive }
    }

    /// The row lifted out of the list as a card of its own (`style::card_lifted`): no gestures, frames or menus of its own, so
    /// the list's drop lines never see it.
    private var liftedFace: some View {
        content.padding(tokens.metrics.borderWidth)
            .background(shape.fill(isSelected ? tokens.sidebarRowSelected : tokens.card))
            .overlay { shape.strokeBorder(isSelected ? tokens.sidebarRowSelected : tokens.cardEdge, lineWidth: tokens.metrics.borderWidth) }
            .shadow(color: tokens.shadow, radius: Space._3, y: Space._1)
            .allowsHitTesting(false)
    }

    private var interactive: some View {
        ZStack(alignment: .topTrailing) {
            Button { context.open(terminal.id) } label: { content }
                .buttonStyle(.plain)
            if hovering && !showsHints && context.renaming != .terminal(terminal.id) {
                CloseButton(isSelected: isSelected) { context.closeTerminal(terminal.id) }
                    .padding(.top, RowGeometry.closeTop).padding(.trailing, inset - (RowGeometry.closeBox - Metric.glyph) / 2)
                    .transition(.opacity)
            }
        }
        // The card's hairline takes room of its own, as the GPUI card's border does: a lone card is 48 + 2 high.
        .padding(solo ? tokens.metrics.borderWidth : 0)
        .background {
            if glass && solo { shape.fill(.clear).glassEffect(.regular, in: shape) }
            shape.fill(fill)
            if let part = row.splitMarks.bracket { SplitLine(part: part, x: inset + metrics.tile / 2) }
        }
        .overlay { if solo { shape.strokeBorder(isSelected ? tokens.sidebarRowSelected : tokens.cardEdge, lineWidth: tokens.metrics.borderWidth) } }
        .overlay { StatusWash(state: tileState, count: reduceMotion ? 0 : context.shifts[terminal.id]?.count ?? 0, enabled: look.statusMotion == .wash, shape: shape) }
        .keyboardFocusRing(keyed == .row(terminal.id), radius: Radius.row)
        .overlay { if let flash = context.flash, flash.id == terminal.id { RowFlash(shape: shape).id(flash.generation) } }
        .contentShape(shape)
        .ringWhileMenuOpen(hovering: hovering)
        .statusNudge(context.shifts[terminal.id])
        .marked("sidebar.row")
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
        .accessibilityAction { context.open(terminal.id) }
        .accessibilityAction(named: terminal.isPinned ? Words.text("sidebar-unpin", fallback: "Unpin") : Words.text("sidebar-pin", fallback: "Pin")) { context.send(.togglePin(terminal.id)) }
        .accessibilityAction(named: Words.text("sidebar-rename-action", fallback: "Rename")) { context.rename(.terminal(terminal.id)) }
        .accessibilityAction(named: Words.text("sidebar-close-terminal", fallback: "Close terminal")) { context.closeTerminal(terminal.id) }
    }

    /// The card's own fill when alone; the row's when it stands in a group's card (whose fill shows through).
    private var fill: Color {
        if isSelected { return tokens.sidebarRowSelected }
        if hovering { return tokens.stateHover }
        return solo && !glass ? tokens.card : .clear
    }

    private var content: some View {
        HStack(spacing: metrics.tileGap) {
            StatusTile(state: tileState, agent: terminal.agent, size: metrics.tile, phase: StatusTile.phase(of: terminal.id),
                       surface: isSelected ? tokens.sidebarRowSelected : tokens.sidebarBackground)
                .statusFlare(context.shifts[terminal.id], tile: metrics.tile)
            RowLines(row: row, isSelected: isSelected, hovering: hovering, rename: rename)
        }
        .padding(.horizontal, inset)
        .padding(.vertical, metrics.density.twoLines ? Space._1 : 0)
        .frame(maxWidth: .infinity, minHeight: metrics.rowHeight, alignment: .leading)
        .contentShape(Rectangle())
    }

    /// With Status dot off every tile is the quiet one (`row/tile.rs` `tile_parts`).
    private var tileState: TileState { look.showsStatus ? TileState(terminal) : .shell }

    /// A rename ends once (↩, ⎋ or a click elsewhere) and the keyboard goes back to the terminal (`SidebarEvent::Renamed`).
    private var rename: RowRename? {
        context.renaming == .terminal(terminal.id) ? RowRename { name in
            context.rename(nil)
            if let name { context.send(.rename(terminal.id, name)) }
            context.hooks.focusTerminal()
        } : nil
    }
}
