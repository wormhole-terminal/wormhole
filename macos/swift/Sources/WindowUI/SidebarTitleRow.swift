// The sidebar's 38 pt row: it leaves room for the traffic lights and the collapse button that float over its start
// holds the profile chip first after them (while there are two profiles or more and the terminals are not shown as tabs,
// whose bar has the chip: `window/chrome.rs` `render_sidebar_header`) and the + that starts a terminal at its end. The terminal's row is PanesUI's `TitleRowView`, between
// paddings that make room for whatever floats at each end (`TitlePads`). The rows hold no toggles of their own:
// those are `WindowControls`, at fixed spots of the window.

import DesignSystem
import PanesUI
import SwiftUI
import WordsModel
import WorkspaceModel

struct SidebarTitleRow: View {
    let facts: WindowFacts
    let send: @MainActor (WorkspaceAction) -> Void
    /// "New terminal with ▸", under a right-click of the +.
    let menu: AnyView
    /// The profiles the chip offers; empty draws none.
    var profiles: [ProfileChoice] = []
    var profileActions = ProfileActions()

    var body: some View {
        HStack(spacing: 0) {
            ProfileChip(choices: profiles, actions: profileActions)
            Spacer(minLength: 0)
            TitleButton(
                label: Words.text("window-new-terminal", fallback: "New terminal"), help: Words.text("window-new-terminal-help", fallback: "New terminal (⌘T)"), stop: 1, motion: .quarter) { color in
                Image(systemName: "plus").font(.system(size: Chrome.glyph, weight: .regular)).foregroundStyle(color)
            } action: {
                send(.newTerminal)
            }
            .contextMenu { menu }
        }
        .padding(.leading, facts.collapseX + Chrome.hit + Chrome.afterToggle)
        .padding(.trailing, Chrome.titleRowPad)
        .frame(height: Chrome.titleHeight)
        .background(WindowDragArea())
    }
}
