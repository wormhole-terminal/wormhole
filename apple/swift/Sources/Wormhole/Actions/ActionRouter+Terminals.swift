// The terminals' own actions: making, closing, pinning and going between them.

import AppKit
import PlatformModel
import WindowUI
import WorkspaceModel

extension ActionRouter {
    func terminalActions() -> [ActionID: Handler] {
        let hub = app.hub
        var table: [ActionID: Handler] = [
            "newTerminal": { hub.send(.newTerminal) },
            "newTerminalNoGroup": {
                hub.send(.newTerminal)
                if let id = hub.state.selected { hub.send(.moveTerminal(id, to: .top(before: nil))) }
            },
            "closeTerminal": { [unowned self] in if let id = hub.state.selected { app.closeTerminal(id) } },
            "closeWindow": { [unowned self] in app.window?.performClose(nil) },
            "pinTerminal": { if let id = hub.state.selected { hub.send(.togglePin(id)) } },
            "nextTerminal": { hub.selectNeighbour(forward: true) },
            "previousTerminal": { hub.selectNeighbour(forward: false) },
            "nextGroup": { hub.selectGroup(step: 1) },
            "previousGroup": { hub.selectGroup(step: -1) },
            "goToWaiting": { hub.goTo(.needsInput) },
            "goToFinished": { hub.goTo(.finished) },
            "copy": { [unowned self] in app.copySelection() },
            "revealInFinder": { hub.selectedTitleActions?.revealFolder() },
            "copyPath": { hub.selectedTitleActions?.copyPath() },
        ]
        for number in 1...9 {
            table[.terminal(number)] = { hub.selectNumbered(number) }
            table[.group(number)] = { hub.selectGroup(number: number) }
        }
        return table
    }

    func paneActions() -> [ActionID: Handler] {
        let hub = app.hub
        return [
            "splitRight": { hub.split(.row) },
            "splitDown": { hub.split(.column) },
            "zoomPane": { hub.toggleZoom() },
            "leaveSplit": { if let id = hub.state.selected { hub.send(.leaveSplit(id)) } },
            "focusPaneLeft": { hub.focusPane(.left) },
            "focusPaneRight": { hub.focusPane(.right) },
            "focusPaneUp": { hub.focusPane(.up) },
            "focusPaneDown": { hub.focusPane(.down) },
        ]
    }
}
