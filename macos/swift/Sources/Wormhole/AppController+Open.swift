// Opening the window and starting everything that goes with it.

import AppKit
import DesignSystem
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
        LegacyLaunch.afterLaunch(dataDirectory: dataDirectory, policy: policy)
        applyLookOptions()
        configureThemes()
        // Settings and the theme reach the terminals before the first one is drawn.
        terminalLook.followTheme()
        let applier = SettingsApplier(app: self, model: settingsModel)
        settingsApplier = applier
        applier.start()
        #if WITH_UNIFFI
            profileFlags.changed = { [weak self] in self?.featuresMoved(in: $0) }
        #endif
        try openSavedWindows()
        menus = MenuController(app: self)
        menus?.start()
        ambient = AmbientController(app: self)
        hotkey = HotkeyController(policy: policy) { [weak self] in self?.ambient?.hotkeyPressed($0) }
        applier.applyAmbient()
        startRealParts()
        if let directory = options.devctl {
            let devctl = DevCtl(directory: directory) { [weak self] line, window in await self?.run(line: line, window: window) }
            try devctl.start()
            self.devctl = devctl
            installGroupAsker()
        }
        if options.onboarding || aux.isFirstRun { aux.showFirstRun() }
        openWhatSystemAsked()
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

    /// Edit ▸ Copy: the focused control's own; a terminal says what it copied on the notice chip (`onCopied`).
    func copySelection() { NSApp.sendAction(#selector(NSText.copy(_:)), to: nil, from: nil) }
}
