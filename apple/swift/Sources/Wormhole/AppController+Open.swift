// Opening the window and starting everything that goes with it.

import AppKit
import PlatformModel
import SettingsModel
import SettingsUI
import TerminalMetal
import WindowUI
import WorkspaceModel

extension AppController {
    func open() throws {
        applyLookOptions()
        configureThemes()
        connectFactory()
        // Settings and the theme reach the terminals before the first one is drawn.
        terminalLook.followTheme()
        let applier = SettingsApplier(app: self, model: settingsModel)
        settingsApplier = applier
        applier.start()
        guard let first = hub.state.selected.flatMap({ hub.terminals.view(for: $0) }) else {
            throw Options.UsageError(description: "no terminal source could start")
        }
        // A fixed grid is a measuring mode: the window fits it. The title row and the status bar sit above and
        // below the terminal: 38 + 24 pt.
        // Otherwise the GPUI app's default window (1320 x 820): both columns open leave the terminal the room its title row needs.
        let size = options.grid.map { grid in
            let terminal = first.renderer.pointSize(forGrid: grid.cols, grid.rows)
            return CGSize(width: terminal.width, height: terminal.height + 62)
        } ?? CGSize(width: 1320, height: 820)
        let controller = WorkspaceWindowController(
            title: "Wormhole (Swift experiment)", contentSize: size, content: WorkspaceRootView(hub: hub, look: look))
        controller.onClose = { [weak self] in self?.hub.terminals.shutdown() }
        controller.onState = { [weak self] active, seen in self?.hub.store.windowState(active: active, seen: seen) }
        windowController = controller
        // The workspace's first word on the window: it is not in front yet (a test instance never is).
        controller.reportState()
        if options.background {
            // A test instance: behind the user's windows, never the active app.
            controller.window?.orderBack(nil)
        } else {
            controller.showWindow(nil)
            NSApp.activate()
        }
        menus = MenuController(app: self)
        menus?.start()
        ambient = AmbientController(app: self)
        applier.applyAmbient()
        startRealParts()
        if let directory = options.devctl {
            let devctl = DevCtl(directory: directory) { [weak self] line in self?.run(command: line) }
            try devctl.start()
            self.devctl = devctl
        }
        if options.onboarding || aux.isFirstRun { aux.showOnboarding(firstRun: aux.isFirstRun) }
        if let seconds = options.idleSeconds { scheduleIdleReport(after: seconds) }
    }

    /// The views' events and what the window asks of the app.
    private func connectFactory() {
        hub.shell = factory.shellName
        factory.onExit = { [weak self] id, code in
            self?.hub.noteExit(of: id, code: code)
            self?.devctl?.append("exited terminal=\(id) code=\(code.map(String.init) ?? "unknown")")
        }
        hub.actions.closeTerminal = { [weak self] in self?.closeTerminal($0) }
        hub.actions.closeGroup = { [weak self] in self?.closeGroup($0) }
        hub.store.onNotice = { [weak self] in self?.answer($0) }
        hub.actions.terminalMenu = { [weak self] in self?.terminalMenu(for: $0) }
        hub.actions.perform = { [weak self] in self?.router.run(ActionID(rawValue: $0)) }
        hub.actions.openSettings = { [weak self] in self?.showSettings() }
        hub.actions.showTerminalsInSidebar = { [weak self] in self?.look.tabsOnTop = false }
    }

    /// What the core asked of the shell that no snapshot carries. The Swift app saves no terminal text, so there is nothing
    /// to erase; a window of its own for a group is not opened yet: it is logged and the group stays where it is.
    private func answer(_ notice: WorkspaceNotice) {
        switch notice {
        case .savedTextForgotten(let id): devctl?.append("saved-text-forgotten terminal=\(id) (nothing is saved)")
        case .groupToWindow(let group, let terminals):
            let line = "group-to-window group=\(group) terminals=\(terminals) (opening a window is not implemented: the group stays here)"
            FileHandle.standardError.write(Data((line + "\n").utf8))
            devctl?.append(line)
        }
    }

    /// The core's settings where the window is real, in memory (this run only) elsewhere.
    func showSettings() {
        if settingsController == nil { settingsController = SettingsWindowController(model: settingsModel) }
        settingsController?.show()
    }

    /// What only a real window has: the notes' store, the banners and the agents' questions.
    private func startRealParts() {
        if let panelNotes {
            notesCoordinator = NotesCoordinator(persistence: panelNotes, hub: hub)
            notesCoordinator?.start()
        }
        guard options.isReal else { return }
        notices = NoticeFeed(app: self)
        aux.startAccess()
        #if WITH_UNIFFI
            (panelEffects as? PanelEffects)?.log = { [weak self] in self?.devctl?.append($0) }
        #endif
    }

    func showLogs() {
        let logs = dataDirectory.appendingPathComponent("logs")
        try? FileManager.default.createDirectory(at: logs, withIntermediateDirectories: true)
        NSWorkspace.shared.activateFileViewerSelecting([logs])
    }

    /// Edit ▸ Copy on the terminal's selection says what it copied on the notice chip (never the text itself).
    func copySelection() {
        let board = NSPasteboard.general
        let before = board.changeCount
        NSApp.sendAction(#selector(NSText.copy(_:)), to: nil, from: nil)
        guard window?.firstResponder is MetalTerminalView, board.changeCount != before,
            let text = board.string(forType: .string)
        else { return }
        hub.notices.post(.copied(text: text))
    }
}
