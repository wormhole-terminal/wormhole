// text-file: replies of the control folder and the idle check (dev tooling, English by design), not interface text
// The app's windows, made: a session over its own workspace (the core's, with ids all windows share) and what a window asks
// of the app. Putting one on the screen is `AppController+WindowOpening`, its end `AppController+WindowEnding`; what is
// saved of each and where each workspace comes from is the core's (`WindowsHost`).

import AppKit
import DesignSystem
import PanesUI
import PlatformModel
import WindowUI
import WindowsModel
import WorkspaceModel

#if WITH_UNIFFI
    import WorkspaceBridge
#endif

extension AppController {
    /// What the project's actions ask of a window, performed by the app (real windows only).
    private static func makeEffects() -> AnyObject? {
        #if WITH_UNIFFI
            PanelEffects()
        #else
            nil
        #endif
    }

    /// A window over its own workspace: filled from what was saved or planned for it (`empty`: terminals are about to
    /// move into it). The window itself is `present`ed apart.
    func makeSession(window id: WindowID, empty: Bool = false, in windows: WindowsHost? = nil) -> WindowSession {
        let windowsHost = windows ?? self.windowsHost
        // Before its terminals start: the engine knows the colours of the window's own profile (the first window of another
        // profile in this run makes that profile's look, which tells them), so a program that asks at once is answered right.
        _ = profileLook(for: windowsHost.profile)
        let host = WorkspaceHost.make(options, windows: windowsHost, window: id, empty: empty)
        let factory = TerminalFactory(options: options, source: host.source)
        let effects = Self.makeEffects()
        let panel = PanelBackendsFactory.make(options, store: host.store, policy: policy, effects: effects, profile: windowsHost.profile)
        let hub = WindowHub(store: host.store, terminals: TerminalViews { factory.make($0) }, panelBackends: panel.backends)
        let session = WindowSession(id: id, windows: windowsHost, hub: hub, factory: factory, effects: effects, usage: panel.usage, backends: panel.backends)
        #if WITH_UNIFFI
            // The title row's order of giving way is the core's, wherever the core runs the window.
            if options.isReal { (hub.rowFitting, hub.splitMotion) = (CoreRowFitting(), CoreSplitMotion()) }
            (effects as? PanelEffects)?.hub = hub
            if options.isReal { (effects as? PanelEffects)?.log = { [weak self] in self?.devctl?.append($0) } }
        #endif
        // Before it is dressed: the registry tells the app's own windows from another profile's by their look.
        sessions.append(session)
        dressForProfile(session)
        connect(session)
        followCloseDot(of: session)
        if let persistence = panel.notes {
            session.notes = NotesCoordinator(persistence: persistence, hub: hub)
            session.notes?.start()
        }
        session.saver = WindowSaver(session: session, windows: windowsHost.backend)
        return session
    }

    /// The views' events and what the window asks of the app.
    func connect(_ session: WindowSession) {
        let hub = session.hub
        hub.shell = session.factory.shellName
        session.factory.claimKey = { [weak self, weak hub] key, flags in
            hub.map { self?.claimProjectKey(key, flags, in: $0) ?? false } ?? false
        }
        session.factory.onExit = { [weak self, weak hub] id, code in
            hub?.noteExit(of: id, code: code)
            self?.devctl?.append("exited terminal=\(id) code=\(code.map(String.init) ?? "unknown")")
        }
        hub.actions.closeTerminal = { [weak self, weak session] in self?.closeTerminal($0, in: session) }
        hub.actions.closeGroup = { [weak self, weak session] in self?.closeGroup($0, in: session) }
        hub.store.onNotice = { [weak self, weak session] in
            guard let session else { return }
            self?.answer($0, from: session)
        }
        hub.sidebarLink.rowMenu = { [weak self, weak session] id, frame in
            if let session { self?.openRowMenu(id, at: frame, in: session) }
        }
        hub.sidebarLink.groupMenu = { [weak self, weak session] id, frame in
            if let session { self?.openGroupMenu(id, at: frame, in: session) }
        }
        connectMenus(of: session)
        hub.actions.terminalMenu = { [weak self, weak session] in self?.terminalMenu(for: $0, in: session) }
        hub.actions.perform = { [weak self] in self?.router.run(ActionID(rawValue: $0)) }
        hub.actions.palette = paletteSources(for: session)
        hub.notices.chipOn = { [weak self, weak session] in session.flatMap { self?.noticesOn($0) } ?? true }
        hub.onRestartToUpdate = { [weak self] in self?.updateFollower?.requestRestart() }
        hub.notices.onDismissed = { [weak self] in if case .warned(.updateReady(let version)) = $0 { self?.updateFollower?.closed(version) } }
        hub.launchesApps = policy.touchesSystem
        hub.onShowProjectCommands = { [weak self] in self?.aux.showTrust() }
        hub.notices.onOpenLogs = { [dataDirectory] in ExternalOpen.open(dataDirectory.appendingPathComponent("logs")) }
        hub.actions.openSettings = { [weak self] in self?.showSettings() }
        hub.actions.profiles = profileList(for: session)
        // The status bar counts every window in sight (this one's terminals first), and a click goes to the terminal's own window.
        hub.actions.everyTerminal = { [weak self, weak session] in
            guard let self, let session else { return [] }
            return session.hub.state.all + shown.filter { $0 !== session }.flatMap { $0.hub.state.all }
        }
        hub.actions.showTerminal = { [weak self] in self?.show(terminal: $0) }
        hub.actions.windowProfile = { [weak self, weak session] in
            guard let self, let session else { return nil }
            return profileList(for: session)().first { $0.here && !$0.current }
        }
        hub.actions.profile = profileActions(for: session)
        hub.terminalDrag.tearOff = { [weak self, weak session] in
            guard let self, let session else { return false }
            return tearOffReleased($0, at: $1, from: session)
        }
        hub.terminalDrag.tracking = { [weak self, weak session] id, point in
            guard let self, let session else { return }
            tearChipMoved(id, to: point, in: session)
        }
        hub.actions.showTerminalsInSidebar = { [weak self] in self?.settingsModel.backend.set("terminals_in", to: .string("sidebar")) }
        hub.actions.shells = { [weak self] in
            (self?.settingsModel.host.shellChoices() ?? []).map { ShellOffer(key: $0.key, name: $0.name, isDefault: $0.isDefault) }
        }
        hub.actions.dismissAgentsTip = { [weak self] in self?.dismissAgentsTip() }
        hub.actions.openSettingsAt = { [weak self] in
            self?.showSettings()
            self?.settingsController?.show(section: $0)
        }
        // A sheet opened from the menu counts as much as one a command-line request opened: the control line answers a second
        // request for this window Failed while it is up.
        hub.panel.worktrees.sheetChanged = { [weak self, weak session] up in
            guard let session else { return }
            self?.hooks?.worktreeSheet(window: session.id, up: up)
        }
        hub.actions.newTerminalWith = { [weak self, weak session] key in self?.newTerminal(shellKey: key, in: session) }
    }

    /// What the core asked of the shell that no snapshot carries. The Swift app saves no terminal text, so there is
    /// nothing to erase; a group asked to have a window of its own gets one, its terminals moving in running.
    func answer(_ notice: WorkspaceNotice, from session: WindowSession) {
        switch notice {
        case .hostFallback(let reason): session.hub.notices.post(.warned(.hostFallback), detail: NoticeReason.clean(reason))
        case .moveToApplications: session.hub.notices.post(.warned(.moveToApplications))
        case .hostChanged(let id): session.hub.hostChanged(id)
        case .hostEnded(let id): finishClosing(id, in: session)
        case .savedTextForgotten(let id): devctl?.append("saved-text-forgotten terminal=\(id) (its saved text was erased)")
        case .groupToWindow(let group, let terminals):
            if !moveToNewWindow(terminals, from: session) {
                devctl?.append("group-to-window group=\(group) terminals=\(terminals) (this workspace cannot hand terminals over: the group stays)")
            }
        }
    }

    /// The window that holds terminal `id`.
    func session(holding id: TerminalID) -> WindowSession? { sessions.first { $0.holds(id) } }

    /// Select terminal `id` in its window and bring that window forward (a test instance keeps it where it is).
    func show(terminal id: TerminalID) {
        guard let holder = session(holding: id) else { return }
        holder.hub.send(.select(id))
        if policy.touchesSystem {
            NSApp.activate()
            holder.window?.makeKeyAndOrderFront(nil)
        }
    }
}
