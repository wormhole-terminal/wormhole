import PlatformModel
import Testing

/// The menu bar from the table, against core's own designed output.
@Suite struct MenuBarTests {
    static func frontWindow() -> MenuState {
        var state = MenuState()
        state.off = ["toggleStatusBar", "toggleMinimap", "fullScreen", "zoomPane"]
        state.checked = ["showChanges"]
        state.terminals = ["sidebar polish", "diff perf", "zsh", "auth tokens", "rate limits"]
        state.currentTerminal = 0
        state.profiles = ["Default", "Work"]
        state.currentProfile = 0
        state.windowProfile = 0
        state.debug = true
        return state
    }

    private let builder = MenuBuilder()

    @Test func theMenusAreTheDesignedOnes() {
        #expect(MenuDump.text(builder.menus(for: Self.frontWindow())) == MenuBarExpected.frontWindow)
    }

    @Test func theDeveloperMenuIsDebugOnly() {
        var state = Self.frontWindow()
        state.debug = false
        let titles = builder.menus(for: state).map(\.title)
        #expect(titles == ["Wormhole", "Shell", "Edit", "View", "Terminal", "Window", "Help"])
    }

    @Test func quitAndEndAllNeedsPersistentTerminals() {
        var state = Self.frontWindow()
        #expect(!MenuDump.text(builder.menus(for: state)).contains("Quit and End All Terminals"))
        state.persistent = true
        #expect(MenuDump.text(builder.menus(for: state)).contains("  Quit Wormhole  ⌘Q\n  Quit and End All Terminals\n"))
    }

    @Test func aSplitClosesAPaneAndUndoSaysWhatItUndoes() {
        var state = Self.frontWindow()
        state.split = true
        state.undo = "Change “primary”"
        let text = MenuDump.text(builder.menus(for: state))
        #expect(text.contains("  Close Pane  ⌘W\n"))
        #expect(text.contains("  Undo Change “primary”  ⌘Z\n"))
        #expect(text.contains("  Redo  ⇧⌘Z\n"))
    }

    @Test func hiddenAndDisabledActionsFollowTheWindow() {
        var state = Self.frontWindow()
        state.hidden = ["historyShow", "showHistory", "newNote"]
        state.disabled = ["revealInFinder", "copyPath"]
        let text = MenuDump.text(builder.menus(for: state))
        #expect(!text.contains("History  ⌃⌘2") && !text.contains("New Note"))
        #expect(text.contains("Reveal in Finder  (disabled)\n") && text.contains("Copy Path  (disabled)\n"))
    }

    @Test func dynamicPartsListWhatTheWindowHas() {
        var state = Self.frontWindow()
        state.recent = ["terminal", "web"]
        state.shells = ["zsh", "fish"]
        state.projectActions = [.init(id: "test", title: "test")]
        let text = MenuDump.text(builder.menus(for: state))
        #expect(text.contains("  Open Recent ▸\n    terminal\n    web\n"))
        #expect(text.contains("  New Terminal With ▸\n    zsh\n    fish\n"))
        #expect(text.contains("  Project Actions ▸\n    test\n"))
    }

    @Test func aRenamedTerminalIsAChangeOfNamesOnly() {
        let before = Self.frontWindow()
        var after = before
        after.terminals[1] = "diff perf (working)"
        #expect(before.differsOnlyInNames(from: after))
        after.currentTerminal = 1
        #expect(!before.differsOnlyInNames(from: after))
        #expect(!before.differsOnlyInNames(from: before))
    }

    @Test func everyItemIsALiveActionAndSeparatorsAreTidy() {
        func walk(_ nodes: [MenuNode]) {
            #expect(nodes.first != .separator && nodes.last != .separator)
            for pair in zip(nodes, nodes.dropFirst()) { #expect(!(pair.0 == .separator && pair.1 == .separator)) }
            for node in nodes {
                switch node {
                case .item(let item):
                    if case .action(let id) = item.command { #expect(Keymap.bundled.entry(id)?.isLive == true, "\(id)") }
                case .submenu(_, let inner, _, _): walk(inner)
                default: break
                }
            }
        }
        var state = Self.frontWindow()
        state.persistent = true
        for menu in builder.menus(for: state) { walk(menu.items) }
    }
}
