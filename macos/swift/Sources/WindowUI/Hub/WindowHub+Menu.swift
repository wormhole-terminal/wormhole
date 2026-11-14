// The menu bar's view of this window: `MenuStateMaker`'s facts gathered from the hub and the window's look.

import PanesModel
import PlatformModel
import WorkspaceModel

extension WindowHub {
    /// What the menus show now. Reading it registers what it reads with the caller's observation, so the menu
    /// bar redraws when any of it changes.
    public func menuState(look: WindowLook, isFullScreen: Bool, implemented: Set<ActionID>, flagsOn: Set<String> = []) -> MenuState {
        let panes = paneFacts
        let profiles = actions.profiles()
        var menu = MenuStateMaker.make(
            MenuFacts(
                state: state, statusBar: look.ownStatusBar, tabsOnTop: look.tabsOnTop, isFullScreen: isFullScreen,
                isSplit: panes.isSplit, isZoomed: panes.isZoomed, canSplitRight: canSplit(.row), canSplitDown: canSplit(.column), minimap: look.ownMinimap, zen: look.zen != nil,
                implemented: implemented, flagsOn: flagsOn, hasFolds: selectedOverlay?.hasFolds ?? false, hasBlocks: selectedOverlay?.hasBlocks ?? false,
                profiles: profiles.map(\.name), currentProfile: profiles.firstIndex { $0.current }, windowProfile: profiles.firstIndex { $0.here }, offeredTabs: panelTabs))
        // The Usage tab's line goes with the tab (no agent on screen); the Project tab's needs a folder (`features/*/def.rs` `menu`).
        if !agentOnScreen { menu.hidden.insert("showUsage") }
        if state.selectedTerminal?.folder == nil { menu.disabled.insert("showProject") }
        // ⌥⌘↑ and ⌥⌘↓ are Select Pane in a terminal and Move Item in a list (`table.rs`: Terminal and List scopes). The menu would
        // take the key before a row's own handler does, so while a list or a note's editor holds the keyboard the two pane items
        // are off and the key reaches the row, as GPUI's focus scope decides.
        if panel.keyboard.listFocused || panel.notes.isWriting { menu.disabled.formUnion(["focusPaneUp", "focusPaneDown"]) }
        return menu
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

    /// ⌃⌥1–9 and ⌃⌥]/[: the tab set with that number in the switcher's list, or the one a step from the selected terminal's
    /// (Ungrouped last, folded groups included, empty ones passed over), in both layouts; it opens as the switcher opens it.
    public func selectGroup(number: Int) {
        if let at = TabSets.numbered(tabBar.sets, number) { openSet(at: at) }
    }

    public func selectGroup(step: Int) {
        let sets = tabBar.sets
        guard let current = state.selected.flatMap({ TabSets.current(sets, containing: $0) }),
              let at = TabSets.step(sets, from: current, forward: step > 0) else { return }
        openSet(at: at)
    }
}
