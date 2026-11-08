// What the menu bar shows for the window in front (`window/menu_state.rs`): which items flip, are checked or
// disabled, the numbered terminals. Pure, so a test reads it without a window. An action the app has no
// implementation for yet is greyed, never left out; a hidden one is an action whose feature is off.

import Foundation
import PlatformModel
import WorkspaceModel

struct MenuFacts {
    var state: WorkspaceState
    var statusBar: Bool
    var tabsOnTop: Bool
    var isFullScreen: Bool
    var isSplit: Bool
    var isZoomed: Bool
    var canSplitRight: Bool
    var canSplitDown: Bool
    var minimap = false
    var zen = false
    /// The actions the dispatcher can run.
    var implemented: Set<ActionID>
}

enum MenuStateMaker {
    static func make(_ facts: MenuFacts, keymap: Keymap = .bundled) -> MenuState {
        var menu = MenuState()
        let state = facts.state
        let selected = state.selectedTerminal
        // The ones that flip their title by what they would do (their thing is off now).
        if !state.sidebarVisible { menu.off.insert("toggleSidebar") }
        if !state.panelVisible { menu.off.insert("toggleRightPanel") }
        if !facts.statusBar { menu.off.insert("toggleStatusBar") }
        if !facts.tabsOnTop { menu.off.insert("toggleTerminalTabs") }
        if !facts.isFullScreen { menu.off.insert("fullScreen") }
        if !facts.isZoomed { menu.off.insert("zoomPane") }
        // Zen mode hides the status bar, the minimap and the panel whatever they are set to: their toggles have nothing to
        // do until it ends (and the menu says which way Zen mode itself goes).
        if facts.zen {
            menu.disabled.formUnion(["toggleStatusBar", "toggleMinimap", "toggleRightPanel"])
        } else {
            menu.off.insert("toggleZenMode")
        }
        if !facts.minimap { menu.off.insert("toggleMinimap") }
        menu.checked.insert(tabAction(state.panelTab))
        if selected?.isPinned == true { menu.checked.insert("pinTerminal") }
        menu.split = facts.isSplit

        menu.disabled.formUnion(keymap.entries.filter { $0.isLive && !facts.implemented.contains($0.id) }.map(\.id))
        if selected == nil { menu.disabled.formUnion(["closeTerminal", "pinTerminal", "splitRight", "splitDown"]) }
        if selected?.folder == nil { menu.disabled.formUnion(["revealInFinder", "copyPath"]) }
        if !facts.canSplitRight { menu.disabled.insert("splitRight") }
        if !facts.canSplitDown { menu.disabled.insert("splitDown") }
        if !facts.isSplit { menu.disabled.formUnion(["zoomPane", "leaveSplit", "focusPaneLeft", "focusPaneRight", "focusPaneUp", "focusPaneDown"]) }
        // View > Layout: the layouts the area has room for, the one in use ticked.
        for preset in state.layout?.menu.presets ?? [] {
            let id = preset.id.actionID
            if preset.current { menu.checked.insert(id) }
            if !preset.enabled { menu.disabled.insert(id) }
        }
        // Actions whose feature flag is off are not in the menus at all.
        menu.hidden = Set(keymap.entries.filter { $0.flag != nil }.map(\.id))

        let numbered = SidebarNumbering.numbered(state)
        menu.terminals = numbered.map(\.title)
        menu.currentTerminal = numbered.firstIndex { $0.id == state.selected }
        return menu
    }

    private static func tabAction(_ tab: PanelTab) -> ActionID {
        switch tab {
        case .changes: "showChanges"
        case .history: "showHistory"
        case .notes: "showNotes"
        case .usage: "showUsage"
        case .project: "showProject"
        case .files: "showFiles"
        }
    }
}

extension LayoutPresetID {
    /// The keymap's id of View > Layout's item for it.
    var actionID: ActionID {
        switch self {
        case .single: "layoutSingle"
        case .twoColumns: "layoutColumns"
        case .twoRows: "layoutRows"
        case .grid: "layoutGrid"
        }
    }
}

/// The terminals ⌘1–9 reach: the numbers the core gave the list's rows, in order.
enum SidebarNumbering {
    static func numbered(_ state: WorkspaceState) -> [TerminalSummary] {
        state.sections.flatMap(\.rows).compactMap { row in row.number.map { ($0, row.terminal) } }
            .sorted { $0.0 < $1.0 }.map(\.1)
    }
}
