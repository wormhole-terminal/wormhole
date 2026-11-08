// The sidebar's collapse button and the right panel's toggle. They float over the title rows at fixed spots of
// the window, so they never move as the columns open and close (and never under the pointer): the collapse
// button 12 pt after the traffic lights, the panel toggle with its glyph on the 12 pt right edge. Each glyph
// fills its strip while its column is in and follows the column's slide.

import SwiftUI
import WordsModel
import WorkspaceModel

struct WindowControls: View {
    let facts: WindowFacts
    let sidebarOpen: Bool
    let sidebarShown: CGFloat
    let panelOpen: Bool
    let panelShown: CGFloat
    /// Wanted but without room: the toggle wears a dot that says it comes back.
    let panelWaiting: Bool
    /// Under Zen mode the panel's button is the way out: it says so, and a click leaves Zen mode instead of opening a panel.
    let leaveZen: (() -> Void)?
    let send: @MainActor (WorkspaceAction) -> Void

    var body: some View {
        // Centred in the 38 pt row, as the GPUI app's buttons are.
        ZStack(alignment: .leading) {
            TitleButton(label: sidebarOpen ? "Hide sidebar" : "Show sidebar", help: sidebarHelp, stop: 0) { color in
                PanelGlyph(side: .left, fill: sidebarShown, color: color)
            } action: {
                send(.toggleSidebar)
            }
            .padding(.leading, facts.collapseX)

            TitleButton(label: panelOpen ? "Hide right panel" : "Show right panel", help: panelHelp, glyph: { color in
                PanelGlyph(side: .right, fill: panelShown, color: color)
            }, marked: panelWaiting) {
                if let leaveZen { leaveZen() } else { send(.togglePanel) }
            }
            .frame(maxWidth: .infinity, alignment: .trailing)
            .padding(.trailing, Chrome.panelToggleRight)
        }
        .frame(height: Chrome.titleHeight)
        .frame(maxHeight: .infinity, alignment: .top)
    }

    private var sidebarHelp: String { (sidebarOpen ? "Hide sidebar" : "Show sidebar") + " (⌘B)" }

    private var panelHelp: String {
        if leaveZen != nil { return Words.text("zen-panel-hidden", fallback: "Zen mode hides the right panel · Click to leave Zen mode") }
        let action = panelOpen ? "Hide right panel" : "Show right panel"
        return (panelWaiting ? action + " · hides the sidebar to make room" : action) + " (⌥⌘B)"
    }
}
