// The first run as a program in a terminal (`Wormhole welcome`, the core's `onboarding::welcome`; the GPUI app's
// `window/welcome_host.rs`). The app starts a terminal whose program is `Wormhole welcome -- <the shell>` with a one-time token
// in that terminal's environment alone, and the core answers the program's requests (`HookBridge`, `hook_server/welcome`); what
// only the window can do arrives as hook actions (`HookHost+Welcome`). Behind the `terminal_welcome` flag: without it the first
// run and Help ▸ Welcome to Wormhole are the intro window.

import AppKit
import AuxModel
#if WITH_UNIFFI
    import SettingsBridge
    import WindowUI
    import WorkspaceBridge
#endif

extension AuxController {
    /// Whether the welcome is a program in a terminal in this run: the flag is on and the core serves the socket.
    var welcomeIsProgram: Bool {
        #if WITH_UNIFFI
            app.hooks?.handle.welcomeAvailable ?? false
        #else
            false
        #endif
    }

    /// Help ▸ Welcome to Wormhole: the terminal welcome when it is on, else the intro window again.
    func showWelcome() {
        if welcomeIsProgram { playWelcome(.cursor) } else { showOnboarding(firstRun: false) }
    }

    /// The first launch (or `--onboarding`): the welcome's terminal, else the intro window.
    func showFirstRun() {
        if welcomeIsProgram {
            playWelcome(.cursor, firstRun: backends.shouldShowWelcome())
        } else {
            showOnboarding(firstRun: isFirstRun)
        }
    }

    /// A new terminal in the front window runs the welcome, if it is on: its token (the core's, for that terminal's
    /// environment alone) is asked for as the terminal starts. A first run's plain terminal, untouched, is the one it replaces.
    @discardableResult
    func startWelcomeTerminal(_ handover: Handover, firstRun: Bool) -> Bool {
        #if WITH_UNIFFI
            guard welcomeIsProgram, let hooks = app.hooks, let backend = app.hub.store.hooks as? RustWorkspaceBackend else { return false }
            let animates = introPlayer.progress != nil
            let reduceMotion = NSWorkspace.shared.accessibilityDisplayShouldReduceMotion
            let importHome = ProcessInfo.processInfo.environment["WORMHOLE_IMPORT_HOME"]
            let before = app.hub.state.all.map(\.id)
            let panelOpen = app.hub.state.panelVisible
            var started: Int?
            backend.onClosed = { [weak hooks] in hooks?.terminalClosed($0) }
            backend.welcomeToken = { id in
                started = id
                return hooks.handle.beginWelcome(
                    tab: Int64(id), handover: handover.rawValue, firstRun: firstRun, animates: animates,
                    reduceMotion: reduceMotion, panelOpen: panelOpen, importHome: importHome,
                    fonts: CoreTextFamilies())
            }
            app.hub.send(.newTerminal)
            backend.welcomeToken = nil
            guard let tab = started else { return false }
            hooks.tellPermission()
            if animates {
                hooks.handle.welcomePlaying(tab: Int64(tab), true)
                introPlayer.onFinish = { [weak hooks] in hooks?.handle.welcomePlaying(tab: Int64(tab), false) }
            }
            if firstRun, before.count == 1, let plain = before.first { app.hub.send(.close(plain)) }
            return true
        #else
            return false
        #endif
    }
}
