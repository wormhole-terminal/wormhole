// The menu bar's view of this window: `MenuStateMaker`'s facts gathered from the hub and the window's look.

import PlatformModel
import WorkspaceModel

extension WindowHub {
    /// What the menus show now. Reading it registers what it reads with the caller's observation, so the menu
    /// bar redraws when any of it changes.
    public func menuState(look: WindowLook, isFullScreen: Bool, implemented: Set<ActionID>, flagsOn: Set<String> = []) -> MenuState {
        let panes = paneFacts
        return MenuStateMaker.make(
            MenuFacts(
                state: state, statusBar: look.ownStatusBar, tabsOnTop: look.tabsOnTop, isFullScreen: isFullScreen,
                isSplit: panes.isSplit, isZoomed: panes.isZoomed, canSplitRight: canSplit(.row), canSplitDown: canSplit(.column), minimap: look.ownMinimap, zen: look.zen != nil,
                implemented: implemented, flagsOn: flagsOn, hasFolds: selectedOverlay?.hasFolds ?? false))
    }

    /// ⌘1–9: the terminal with this number in the list (the core numbers them).
    public func selectNumbered(_ number: Int) { send(.selectNumber(number)) }

    /// ⌃⌘↓ / ⌃⌘↑: the next or previous terminal in the list, round the ends (the core walks the list).
    public func selectNeighbour(forward: Bool) { send(.selectStep(forward ? 1 : -1)) }

    /// ⌥⌘J, ⇧⌥⌘J: the terminal that waits for you, or has finished.
    public func goTo(_ activity: Activity) {
        let order = state.all
        let after = state.selected.flatMap { id in order.firstIndex { $0.id == id } } ?? -1
        let found = (order.dropFirst(after + 1) + order.prefix(max(after, 0))).first { $0.activity == activity }
        if let found { send(.select(found.id)) }
    }

    /// ⌃⌥]/[ and ⌃⌥1–9: the first terminal of the group `offset` steps from the selected terminal's, or number `n`.
    /// A folded group lists no rows, so it has none to go to until it is opened.
    public func selectGroup(number: Int) {
        let groups = groupSections
        guard groups.indices.contains(number - 1), let first = groups[number - 1].rows.first else { return }
        send(.select(first.id))
    }

    public func selectGroup(step: Int) {
        let groups = groupSections
        guard !groups.isEmpty else { return }
        let current = groups.firstIndex { $0.rows.contains { $0.id == state.selected } } ?? (step > 0 ? -1 : 0)
        let next = (current + step + groups.count) % groups.count
        if let first = groups[next].rows.first { send(.select(first.id)) }
    }

    private var groupSections: [SidebarSection] { state.sections.filter { $0.head != nil } }
}
