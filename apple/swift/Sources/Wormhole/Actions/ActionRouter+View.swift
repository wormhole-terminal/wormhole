// The View menu: the columns, the panel's tabs, the palette and how the window looks.

import AppKit
import PlatformModel
import WindowUI
import WorkspaceModel

extension ActionRouter {
    func viewActions() -> [ActionID: Handler] {
        var hub: WindowHub { app.hub }
        let look = app.look
        return [
            "toggleSidebar": { hub.send(.toggleSidebar) },
            // Under Zen mode these have nothing to show and would only change what comes back, unseen: the menu greys them.
            "toggleRightPanel": { if look.zen == nil { hub.send(.togglePanel) } },
            "toggleStatusBar": { if look.zen == nil { look.statusBar.toggle() } },
            "toggleZenMode": { [unowned self] in app.toggleZen() },
            "toggleTerminalTabs": { look.tabsOnTop.toggle() },
            "focusNextArea": { hub.focus.step(1) },
            "focusPreviousArea": { hub.focus.step(-1) },
            "focusTerminalOrPanel": { hub.focus.toggleTerminalAndPanel() },
            "showChanges": { hub.send(.showPanelTab(.changes)) },
            "showHistory": { hub.send(.showPanelTab(.history)) },
            "showNotes": { hub.send(.showPanelTab(.notes)) },
            "showUsage": { hub.send(.showPanelTab(.usage)) },
            "showProject": { hub.send(.showPanelTab(.project)) },
            "showFiles": { hub.send(.showPanelTab(.files)) },
            "stashPrompt": { if hub.panel.notes.stashPrompt() { hub.send(.showPanelTab(.notes)) } },
            "commandPalette": { hub.send(.showOverlay(hub.state.overlay == .palette ? nil : .palette)) },
            "keyboardShortcuts": { hub.send(.showOverlay(hub.state.overlay == .shortcuts ? nil : .shortcuts)) },
            "bigger": { hub.zoomText(step: 1) },
            "smaller": { hub.zoomText(step: -1) },
            "actualSize": { hub.zoomText(step: nil) },
            // Show / Hide Minimap is the setting itself (Settings and the menu are one switch).
            "toggleMinimap": { [unowned self] in
                guard look.zen == nil else { return }
                let backend = app.settingsModel.backend
                backend.set("minimap", to: .bool(!(backend.values["minimap"]?.bool ?? false)))
            },
        ]
    }

    func findActions() -> [ActionID: Handler] {
        var hub: WindowHub { app.hub }
        return [
            "find": { if !hub.sidebarSearch.revealIfFocused() { hub.find(.open) } },
            "findNext": { hub.find(.next) },
            "findPrevious": { hub.find(.previous) },
            "useSelectionForFind": { hub.find(.useSelection) },
            // The command blocks' Edit menu (Terminal.app's marks): jumps between prompts and the actions on a block.
            "previousPrompt": { hub.jumpPrompt(.up) },
            "nextPrompt": { hub.jumpPrompt(.down) },
            "selectOutput": { hub.blockAction(.selectOutput) },
            "copyOutput": { hub.blockAction(.copyOutput) },
            "copyCommand": { hub.blockAction(.copyCommand) },
            "rerunCommand": { hub.blockAction(.putAtPrompt) },
        ]
    }
}
