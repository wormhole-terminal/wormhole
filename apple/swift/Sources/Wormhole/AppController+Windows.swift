// The app's windows, made: a session over its own workspace (the core's, with ids all windows share) and what a window asks
// of the app. Putting one on the screen is `AppController+WindowOpening`, its end `AppController+WindowEnding`; what is
// saved of each and where each workspace comes from is the core's (`WindowsHost`).

import AppKit
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
        let host = WorkspaceHost.make(options, windows: windowsHost, window: id, empty: empty)
        let factory = TerminalFactory(options: options, source: host.source)
        let effects = Self.makeEffects()
        let panel = PanelBackendsFactory.make(options, store: host.store, policy: policy, effects: effects, profile: windowsHost.profile)
        let hub = WindowHub(store: host.store, terminals: TerminalViews { factory.make($0) }, panelBackends: panel.backends)
        let session = WindowSession(id: id, windows: windowsHost, hub: hub, factory: factory, effects: effects, usage: panel.usage, backends: panel.backends)
        #if WITH_UNIFFI
            // The title row's order of giving way is the core's, wherever the core runs the window.
            if options.program != nil { hub.rowFitting = CoreRowFitting() }
            (effects as? PanelEffects)?.hub = hub
            if options.isReal { (effects as? PanelEffects)?.log = { [weak self] in self?.devctl?.append($0) } }
        #endif
        factory.look = terminalLook
        settingsApplier?.dress(session)
        connect(session)
        if let persistence = panel.notes {
            session.notes = NotesCoordinator(persistence: persistence, hub: hub)
            session.notes?.start()
        }
        session.saver = WindowSaver(session: session, windows: windowsHost.backend)
        sessions.append(session)
        return session
    }

    /// The views' events and what the window asks of the app.
    func connect(_ session: WindowSession) {
        let hub = session.hub
        hub.shell = session.factory.shellName
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
        hub.actions.terminalMenu = { [weak self, weak session] in self?.terminalMenu(for: $0, in: session) }
        hub.actions.perform = { [weak self] in self?.router.run(ActionID(rawValue: $0)) }
        hub.actions.palette = paletteSources(for: session)
        hub.notices.chipOn = { [weak self, weak session] in session.flatMap { self?.noticesOn($0) } ?? true }
        hub.onRestartToUpdate = { [weak self] in self?.updateFollower?.requestRestart() }
        hub.notices.onOpenLogs = { [dataDirectory] in NSWorkspace.shared.open(dataDirectory.appendingPathComponent("logs")) }
        hub.actions.openSettings = { [weak self] in self?.showSettings() }
        hub.actions.profiles = profileList(for: session)
        hub.actions.profile = profileActions(for: session)
        hub.terminalDrag.tearOff = { [weak self, weak session] in
            guard let self, let session else { return false }
            return tearOffReleased($0, at: $1, from: session)
        }
        hub.terminalDrag.tracking = { [weak self, weak session] id, point in
            guard let self, let session else { return }
            tearChipMoved(id, to: point, in: session)
        }
        hub.actions.showTerminalsInSidebar = { [weak self] in self?.look.tabsOnTop = false }
    }

    /// What the core asked of the shell that no snapshot carries. The Swift app saves no terminal text, so there is
    /// nothing to erase; a group asked to have a window of its own gets one, its terminals moving in running.
    func answer(_ notice: WorkspaceNotice, from session: WindowSession) {
        switch notice {
        case .hostFallback(let reason): session.hub.notices.post(.warned(.hostFallback), detail: NoticeReason.clean(reason))
        case .moveToApplications: session.hub.notices.post(.warned(.moveToApplications))
        case .hostChanged(let id): session.hub.hostChanged(id)
        case .hostEnded(let id): finishClosing(id, in: session)
        case .savedTextForgotten(let id): devctl?.append("saved-text-forgotten terminal=\(id) (nothing is saved)")
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
        if policy.touchesSystem { holder.window?.makeKeyAndOrderFront(nil) }
    }
}
