// The sidebar's 38 pt row: it leaves room for the traffic lights and the collapse button that float over its start
// and holds the + that starts a terminal at its end. The terminal's row is PanesUI's `TitleRowView`, between
// paddings that make room for whatever floats at each end (`TitlePads`). The rows hold no toggles of their own:
// those are `WindowControls`, at fixed spots of the window.

import DesignSystem
import SwiftUI
import WorkspaceModel

struct SidebarTitleRow: View {
    let facts: WindowFacts
    let send: @MainActor (WorkspaceAction) -> Void

    var body: some View {
        HStack(spacing: 0) {
            Spacer(minLength: 0)
            TitleButton(label: "New terminal", help: "New terminal (⌘T)", stop: 1) { color in
                Image(systemName: "plus").font(.system(size: Chrome.glyph, weight: .regular)).foregroundStyle(color)
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
