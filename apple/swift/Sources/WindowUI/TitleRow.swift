// The 38 pt rows across the top. The sidebar's leaves room for the traffic lights and the collapse button that
// float over its start and holds the + that starts a terminal at its end; the centre's carries the selected
// terminal's breadcrumb between paddings that make room for whatever floats at each end (`TitlePads`). The
// rows hold no toggles of their own: those are `WindowControls`, at fixed spots of the window.

import DesignSystem
import SwiftUI
import WorkspaceModel

struct SidebarTitleRow: View {
    let facts: WindowFacts
    let send: @MainActor (WorkspaceAction) -> Void

    var body: some View {
        HStack(spacing: 0) {
            Spacer(minLength: 0)
            TitleButton(label: "New terminal", help: "New terminal (⌘T)") { color in
                Image(systemName: "plus").font(.system(size: Chrome.glyph - 2, weight: .medium)).foregroundStyle(color)
            } action: {
                send(.newTerminal)
            }
        }
        .padding(.leading, facts.collapseX + Chrome.hit + Chrome.afterToggle)
        .padding(.trailing, Chrome.titleRowPad)
        .frame(height: Chrome.titleHeight)
        .background(WindowDragArea())
    }
}

struct CentreTitleRow: View {
    let terminal: TerminalSummary?
    let pads: TitlePads
    let showsState: Bool

    var body: some View {
        HStack(spacing: 0) {
            if let terminal { Breadcrumb(terminal: terminal, showsState: showsState) }
            Spacer(minLength: 0)
        }
        .padding(.leading, pads.leading)
        .padding(.trailing, pads.trailing)
        .frame(height: Chrome.titleHeight)
        .background(WindowDragArea())
    }
}
