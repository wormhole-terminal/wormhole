// The terminals as tabs, instead of the sidebar (Settings → Show terminals in → Tabs), as RF-02 draws it: one
// 28 pt row under the title row with the profile chip (while there are two profiles or more), the group
// switcher, the current group's terminals as tabs and the +. None while there is only one terminal and no
// group to switch to. The row of tabs scrolls sideways once they are at their narrowest, and brings the
// selected one into view.

import DesignSystem
import PanesModel
import SwiftUI
import WorkspaceModel

public struct TabBarView: View {
    let model: TabBarModel
    let profiles: [ProfileChoice]
    let profileActions: ProfileActions
    let actions: TabBarActions
    /// ⌘ is held: the tabs show their numbers instead of the ×.
    let hints: Bool
    @State private var switcherOpen = false
    @Environment(\.tokens) private var tokens
    @Environment(\.reduceMotion) private var reduceMotion

    public init(
        model: TabBarModel, profiles: [ProfileChoice] = [], profileActions: ProfileActions = ProfileActions(),
        actions: TabBarActions = TabBarActions(), hints: Bool = false
    ) {
        (self.model, self.profiles, self.profileActions, self.actions, self.hints) = (model, profiles, profileActions, actions, hints)
    }

    public var body: some View {
        if model.isShown {
            HStack(spacing: Space._2) {
                if profiles.count > 1 {
                    ProfileChip(choices: profiles, actions: profileActions)
                    rule
                }
                if model.showsSwitcher {
                    GroupSwitcherButton(label: model.setLabel, alert: model.alert, open: $switcherOpen)
                        .popover(isPresented: $switcherOpen, arrowEdge: .bottom) {
                            GroupSwitcherView(model: model, actions: actions) { switcherOpen = false }
                        }
                    rule
                }
                tabs
                IconButton(symbol: "plus", label: LocalizedStringKey(PanesText.newTerminal)) { actions.newTerminal(currentGroup) }
                    .contextMenu { actions.newMenu() }
            }
            .padding(.horizontal, Metric.edgeInset)
            .frame(height: TabMetrics.height)
            .transition(.opacity)
        }
    }

    private var tabs: some View {
        ScrollViewReader { reader in
            ScrollView(.horizontal, showsIndicators: false) {
                HStack(spacing: Space._1) {
                    ForEach(model.tabs) { tab in
                        TerminalTabView(tab: tab, selected: tab.id == model.selected, hints: hints, tile: actions.tile?(tab.id, TabMetrics.tile)) { actions.close(tab.id) }
                            .onTapGesture { actions.select(tab.id) }
                            .contextMenu { actions.tabMenu(tab.id) }
                            .id(tab.id)
                    }
                }
            }
            .onChange(of: model.selected) { _, id in
                guard let id else { return }
                withAnimation(reduceMotion ? nil : Motion.selection) { reader.scrollTo(id) }
            }
            .accessibilityElement(children: .contain)
            .accessibilityAddTraits(.isTabBar)
        }
    }

    private var rule: some View {
        Rectangle().fill(tokens.border).frame(width: tokens.metrics.borderWidth, height: TabMetrics.ruleHeight)
    }

    private var currentGroup: GroupID? { model.sets.first { $0.members.contains { $0 == model.selected } }?.groupID }
}
