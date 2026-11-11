// Opening the window and starting everything that goes with it.

import AppKit
import PlatformUI
import AuxModel
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
        // Settings and the theme reach the terminals before the first one is drawn.
        terminalLook.followTheme()
        let applier = SettingsApplier(app: self, model: settingsModel)
        settingsApplier = applier
        applier.start()
        try openSavedWindows()
        menus = MenuController(app: self)
        menus?.start()
        ambient = AmbientController(app: self)
        hotkey = HotkeyController(policy: policy) { [weak self] in self?.ambient?.hotkeyPressed($0) }
        applier.applyAmbient()
        startRealParts()
        if let directory = options.devctl {
            let devctl = DevCtl(directory: directory) { [weak self] line in await self?.run(line: line) }
            try devctl.start()
            self.devctl = devctl
        }
        if options.onboarding || aux.isFirstRun { aux.showOnboarding(firstRun: aux.isFirstRun) }
        if let seconds = options.idleSeconds { scheduleIdleReport(after: seconds) }
    }

    /// The core's settings where the window is real, in memory (this run only) elsewhere.
    func showSettings() {
        if settingsController == nil { settingsController = SettingsWindowController(model: settingsModel, updates: UpdateViewModel(backend: aux.updateBackend())) }
        settingsController?.show()
    }

    /// What only a real run has: the banners, the agents' questions and the socket the agents' hooks reach.
    private func startRealParts() {
        guard options.isReal else { return }
        notices = NoticeFeed(app: self)
        aux.startAccess()
        startHooks()
        startUpdates()
        #if WITH_UNIFFI
            let follower = ProjectSettingsFollower(app: self)
            follower?.start()
            projectSettings = follower
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
