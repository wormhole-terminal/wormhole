// The terminal's column: its colour from the title row down, the title row across the top and the terminal area
// as a rounded surface below it, inset 8 pt from the column's sides and bottom so the grid's text (4 pt of
// padding) stands on the window's 12 pt edge and lines up with the traffic lights. The inset also keeps a program
// that paints every cell clear of the window's own corners. The surface is clipped, never sized, while a column
// slides over it. An open diff or file of the right panel takes the title row and the surface in turn
// (`TerminalStage`, `TitleSlot`); the panes stay alive under it.

import DesignSystem
import PanelUI
import PanesModel
import PanesUI
import SidebarUI
import SwiftUI
import TerminalUI
import WorkspaceModel

struct TerminalColumn: View {
    let hub: WindowHub
    let pads: TitlePads
    @Environment(\.chrome) private var colours
    @Environment(WindowLook.self) private var look
    @Environment(WindowFocus.self) private var focus
    @State private var hold = CommandHoldMonitor()

    /// ⌘ held on its own shows the numbers on the tabs, as it does on the rows (`sidebar_cmd_hints`).
    private var hints: Bool { look.sidebar.forcesHints || (look.sidebar.showsCommandHints && hold.showing) }

    var body: some View {
        let cover = hub.panel.cover
        VStack(spacing: 0) {
            TitleSlot {
                CentreTitle(hub: hub)
            } coverTitle: {
                PanelCoverTitle.carries(cover) ? PanelCoverTitle(controller: hub.panel) : nil
            }
            // The strip wears the ring while the keys are on it (← → step between terminals).
            .overlay { Color.clear.padding(.vertical, Space._1).focusRing(focus.ring(.tabs), radius: colours.radius) }
            .padding(.leading, pads.leading)
            .padding(.trailing, pads.trailing)
            .frame(height: Chrome.titleHeight)
            // A group's colour (a named one or the person's own) tints the row: its soft parts step up to the text colour,
            // and under Increase Contrast a line says the colour.
            .environment(\.titleTinted, hub.state.selectedGroupHead.map { $0.color != nil || $0.customColor != nil } ?? false)
            .overlay { TitleTintLine(group: hub.state.selectedGroupHead) }
            .background(WindowDragArea())

            if look.tabsOnTop { TabBarView(model: hub.tabBar, profiles: hub.actions.profiles(), profileActions: hub.actions.profile, actions: hub.tabActions(appearance: look.sidebar), hints: hints) }

            TerminalStage {
                TerminalPanes(hub: hub)
            } cover: {
                cover == nil ? nil : PanelCoverView(controller: hub.panel).environment(\.fileIcons, PanelBody.icons)
            }
            // At the window's opacity, as the terminal's own background is (GPUI paints the pane translucent over the blur).
            .background(look.surface(colours.terminalSurface))
            .overlay { NoticeChipHost(center: hub.notices) }
            .clipShape(RoundedRectangle(cornerRadius: colours.radiusLarge))
            .padding([.horizontal, .bottom], Chrome.surfaceInset)
        }
        .background(look.surface(colours.terminal))
        .onAppear { hold.start() }
        .onDisappear { hold.stop() }
    }
}

/// The selected terminal's breadcrumb and the buttons at the row's other end.
private struct CentreTitle: View {
    let hub: WindowHub
    @Environment(WindowLook.self) private var look
    /// How long the terminal has been in its state, as the core says it; read again when the words would read differently.
    @State private var aged: (since: UInt64, waited: Waited)?

    var body: some View {
        if let terminal = hub.state.selectedTerminal {
            let first = terminal.since.flatMap { hub.store.waited(since: $0) }
            let waited = aged.flatMap { $0.since == terminal.since ? $0.waited : nil } ?? first
            let facts = TitleFactsMaker.facts(
                for: terminal, row: hub.titleRow, statusBarOn: look.statusBar, gauge: hub.gauges.gauge(for: terminal),
                extras: hub.state.title, waited: waited)
            TitleRowView(
                facts: facts, arrangement: hub.state.title.map { TitleArrangement($0, facts: facts) }, fitting: hub.rowFitting,
                actions: hub.titleActions(for: terminal, appearance: look.sidebar), backend: hub.openIn, icon: SystemOpenIn.icon)
                .modifier(TitleFileDrop { hub.terminals.existing(terminal.id)?.insertDropped($0) })
                .task(id: waited) {
                    guard !look.statusBar, let waited, let since = terminal.since else { return }
                    try? await Task.sleep(for: waited.next)
                    if !Task.isCancelled, let fresh = hub.store.waited(since: since) { aged = (since, fresh) }
                }
        } else {
            Color.clear
        }
    }
}
