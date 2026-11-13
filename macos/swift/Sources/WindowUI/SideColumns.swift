// The two side columns' own surfaces: the sidebar's (its title row over the list) and the right panel's (its
// tab bar over the tab). Each paints its translucent surface at the window's opacity over the blur, as at rest
// and while it slides, so the one choice cannot drift between them. On Liquid Glass the sidebar paints none:
// the system's glass lies behind it, its cards float on it.

import SwiftUI
import SidebarUI
import WorkspaceModel

struct SidebarColumn: View {
    let hub: WindowHub
    let facts: WindowFacts
    @Environment(\.chrome) private var colours
    @Environment(WindowLook.self) private var look

    var body: some View {
        VStack(spacing: 0) {
            // The chip is the tab bar's while the terminals are shown as tabs.
            SidebarTitleRow(
                facts: facts, send: hub.send, menu: hub.newTerminalMenu,
                profiles: look.tabsOnTop ? [] : hub.actions.profiles(), profileActions: hub.actions.profile)
            SidebarView(
                state: hub.state, appearance: look.sidebar, hooks: hub.sidebarHooks, link: hub.sidebarLink,
                band: hub.exampleBand, send: hub.send
            )
            .environment(\.dropPlanner, hub.store)
            .environment(\.terminalDrag, hub.terminalDrag)
            // Gone once a terminal of the window runs an agent.
            NoAgentsTip(
                stage: hub.state.all.contains { $0.agent != nil } ? .off : hub.agentsTip,
                openSettings: { hub.actions.openSettingsAt("agents") }, close: { hub.agentsTip = .offer },
                never: { hub.actions.dismissAgentsTip(); hub.agentsTip = .off }, keep: { hub.agentsTip = .off })
        }
        .background { if !look.glassOn { look.surface(colours.sidebar) } }
    }
}

struct PanelColumn: View {
    let hub: WindowHub
    @Environment(\.chrome) private var colours
    @Environment(WindowLook.self) private var look

    var body: some View {
        RightPanel(hub: hub)
            .background(look.surface(colours.panel))
    }
}
