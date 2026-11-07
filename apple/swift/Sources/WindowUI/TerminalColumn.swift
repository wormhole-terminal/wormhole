// The terminal's column: its colour from the title row down, the title row across the top and the terminal as a
// rounded surface below it, inset 8 pt from the column's sides and bottom so the grid's text (4 pt of padding)
// stands on the window's 12 pt edge and lines up with the traffic lights. The inset also keeps a program that
// paints every cell clear of the window's own corners. The surface is clipped, never sized, while a column
// slides over it.

import SwiftUI
import WorkspaceModel

struct TerminalColumn: View {
    let terminal: TerminalSummary?
    let selected: TerminalID?
    let ids: [TerminalID]
    let views: TerminalViews
    let pads: TitlePads
    let showsState: Bool
    @Environment(\.chrome) private var colours
    @Environment(WindowLook.self) private var look

    var body: some View {
        VStack(spacing: 0) {
            CentreTitleRow(terminal: terminal, pads: pads, showsState: showsState)
            TerminalHost(views: views, selected: selected, ids: ids)
                .background(colours.terminalSurface)
                .clipShape(RoundedRectangle(cornerRadius: colours.radiusLarge))
                .padding([.horizontal, .bottom], Chrome.surfaceInset)
        }
        .background(look.surface(colours.terminal))
    }
}
