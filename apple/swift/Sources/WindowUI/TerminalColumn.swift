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
import SwiftUI
import TerminalUI

struct TerminalColumn: View {
    let hub: WindowHub
    let pads: TitlePads
    @Environment(\.chrome) private var colours
    @Environment(WindowLook.self) private var look

    var body: some View {
        let cover = hub.panel.cover
        VStack(spacing: 0) {
            TitleSlot {
                CentreTitle(hub: hub)
            } coverTitle: {
                PanelCoverTitle.carries(cover) ? PanelCoverTitle(controller: hub.panel) : nil
            }
            .padding(.leading, pads.leading)
            .padding(.trailing, pads.trailing)
            .frame(height: Chrome.titleHeight)
            .background(WindowDragArea())

            if look.tabsOnTop { TabBarView(model: hub.tabBar, actions: hub.tabActions) }

            TerminalStage {
                TerminalPanes(hub: hub)
            } cover: {
                cover == nil ? nil : PanelCoverView(controller: hub.panel).environment(\.fileIcons, PanelBody.icons)
            }
            .background(colours.terminalSurface)
            .overlay { NoticeChipHost(center: hub.notices) }
            .clipShape(RoundedRectangle(cornerRadius: colours.radiusLarge))
            .padding([.horizontal, .bottom], Chrome.surfaceInset)
        }
        .background(look.surface(colours.terminal))
    }
}

/// The selected terminal's breadcrumb and the buttons at the row's other end.
private struct CentreTitle: View {
    let hub: WindowHub
    @Environment(WindowLook.self) private var look

    var body: some View {
        if let terminal = hub.state.selectedTerminal {
            let facts = TitleFactsMaker.facts(
                for: terminal, row: hub.titleRow, statusBarOn: look.statusBar, gauge: hub.gauges.gauge(for: terminal))
            TitleRowView(
                facts: facts, arrangement: hub.state.title.map { TitleArrangement($0, facts: facts) }, fitting: hub.rowFitting,
                actions: hub.titleActions(for: terminal), backend: hub.openIn, icon: SystemOpenIn.icon)
        } else {
            Color.clear
        }
    }
}
