// The menu bar's view of this window: `MenuStateMaker`'s facts gathered from the hub and the window's look.

import PlatformModel
import WorkspaceModel

extension WindowHub {
    /// What the menus show now. Reading it registers what it reads with the caller's observation, so the menu
    /// bar redraws when any of it changes.
    public func menuState(look: WindowLook, isFullScreen: Bool, implemented: Set<ActionID>) -> MenuState {
        let panes = paneFacts
        return MenuStateMaker.make(
            MenuFacts(
                state: state, statusBar: look.statusBar, tabsOnTop: look.tabsOnTop, isFullScreen: isFullScreen,
                isSplit: panes.isSplit, isZoomed: panes.isZoomed, canSplitRight: canSplit(.row), canSplitDown: canSplit(.column),
                implemented: implemented))
    }

    /// ⌘1–9: the terminal with this number in the list.
    public func selectNumbered(_ number: Int) {
        let numbered = SidebarNumbering.numbered(state)
        if numbered.indices.contains(number - 1) { send(.select(numbered[number - 1].id)) }
    }

    /// ⌃⌘↓ / ⌃⌘↑: the next or previous terminal in the list, round the ends.
    public func selectNeighbour(forward: Bool) {
        let order = SidebarListing.sections(for: state).flatMap(\.rows).map(\.id)
        guard !order.isEmpty else { return }
        let at = state.selected.flatMap { order.firstIndex(of: $0) } ?? (forward ? -1 : 0)
        send(.select(order[(at + (forward ? 1 : order.count - 1)) % order.count]))
    }

    /// ⌥⌘J, ⇧⌥⌘J: the terminal that waits for you, or has finished.
    public func goTo(_ activity: Activity) {
        let order = SidebarListing.sections(for: state).flatMap(\.rows).map(\.terminal)
        let after = state.selected.flatMap { id in order.firstIndex { $0.id == id } } ?? -1
        let found = (order.dropFirst(after + 1) + order.prefix(max(after, 0))).first { $0.activity == activity }
        if let found { send(.select(found.id)) }
    }

    /// ⌃⌥]/[ and ⌃⌥1–9: the first terminal of the group `offset` steps from the selected terminal's, or number `n`.
    public func selectGroup(number: Int) {
        guard state.groups.indices.contains(number - 1), let first = state.groups[number - 1].members.first else { return }
        send(.select(first))
    }

    public func selectGroup(step: Int) {
        guard !state.groups.isEmpty else { return }
        let current = state.groups.firstIndex { $0.members.contains { $0 == state.selected } } ?? (step > 0 ? -1 : 0)
        let next = (current + step + state.groups.count) % state.groups.count
        if let first = state.groups[next].members.first { send(.select(first)) }
    }
}
