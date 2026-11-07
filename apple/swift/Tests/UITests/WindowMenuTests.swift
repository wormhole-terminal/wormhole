import PlatformModel
import Testing
import WorkspaceModel

@testable import WindowUI

/// What the menu bar shows for a window: flips, checks, greyed actions and the numbered terminals.
@MainActor
@Suite struct WindowMenuTests {
    private func facts(_ state: WorkspaceState, implemented: Set<ActionID> = ["newTerminal", "find"]) -> MenuFacts {
        MenuFacts(
            state: state, statusBar: false, tabsOnTop: false, isFullScreen: false, isSplit: false, isZoomed: false,
            canSplitRight: true, canSplitDown: true, implemented: implemented)
    }

    @Test func anActionWithNoImplementationIsGreyedNeverHidden() {
        let menu = MenuStateMaker.make(facts(MockWorkspace.state))
        #expect(menu.disabled.contains("bigger"))
        #expect(!menu.disabled.contains("newTerminal"))
        #expect(menu.hidden.isDisjoint(with: ["bigger", "newWindow"]))
    }

    @Test func titlesFlipForWhatIsOff() {
        var state = MockWorkspace.state
        state.sidebarVisible = false
        let menu = MenuStateMaker.make(facts(state))
        #expect(menu.off.contains("toggleSidebar"))
        #expect(!menu.off.contains("toggleRightPanel"))
        #expect(menu.checked.contains("showChanges"))
    }

    @Test func thePaneVerbsNeedASplit() {
        let alone = MenuStateMaker.make(facts(MockWorkspace.state))
        #expect(alone.disabled.isSuperset(of: ["zoomPane", "leaveSplit", "focusPaneLeft"]))
        var split = facts(MockWorkspace.state, implemented: ["leaveSplit", "focusPaneLeft"])
        split.isSplit = true
        #expect(MenuStateMaker.make(split).split)
        #expect(MenuStateMaker.make(split).disabled.isDisjoint(with: ["leaveSplit", "focusPaneLeft"]))
    }

    @Test func theNumberedTerminalsFollowTheListsOrder() {
        let state = MockWorkspace.state
        let menu = MenuStateMaker.make(facts(state))
        #expect(menu.terminals.count == 9)
        #expect(menu.terminals.first == "htop")
        #expect(menu.currentTerminal.map { menu.terminals[$0] } == state.selectedTerminal?.title)
    }

    @Test func theMenuBarBuildsFromTheState() {
        let text = MenuDump.text(MenuBuilder().menus(for: MenuStateMaker.make(facts(MockWorkspace.state))))
        #expect(text.contains("New Terminal  ⌘T\n"))
        #expect(text.contains("Bigger  ⌘+  (disabled)"))
    }
}
