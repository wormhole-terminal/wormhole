// The surfaces outside the window that say what the terminals are doing: the menu bar extra, the Dock badge and
// menu, and the hotkey's quick panel. Each is the platform edge's own controller; this feeds them the terminals
// as the workspace has them and carries their choices back. A test instance (`--background`) never makes them:
// nothing of it shows in the menu bar, the Dock or over another app (`PlatformPolicy`).

import AppKit
import Observation
import PlatformModel
import PlatformUI
import WorkspaceModel

@MainActor
final class AmbientController {
    let status: StatusItemController
    private let dock: DockController
    /// What the Dock badge counts (Settings → Notifications → Dock); set by `apply`.
    var badgeMode = DockBadgeMode.needsYou { didSet { if badgeMode != oldValue { dock.update(terminals: listed, badgeMode: badgeMode) } } }
    private var listed: [AmbientTerminal] = []
    /// What the menu bar item shows and whether its working glyph types out (`menu_bar_icon`, `menu_bar_animate`).
    var iconMode = MenuBarIconMode.statusCount { didSet { if iconMode != oldValue { pushStatus() } } }
    var animatesIcon = false { didSet { if animatesIcon != oldValue { pushStatus() } } }
    /// The global key that raises the quick panel or the app, once Settings names one.
    var hotkey: GlobalHotkey?
    private let quick: QuickPanelController
    private unowned let app: AppController

    init?(app: AppController) {
        guard app.policy.touchesSystem else { return nil }
        self.app = app
        status = StatusItemController(policy: app.policy)
        dock = DockController(policy: app.policy)
        quick = QuickPanelController { [weak app] in (app?.ambientTerminals ?? [], [.newTerminal(cwd: nil)]) }
        let route: (MenuCommand) -> Void = { [weak app] in app?.router.dispatch($0) }
        status.dispatcher.handler = route
        dock.dispatcher.handler = route
        quick.model.send = { [weak self] in self?.run($0) }
        follow()
    }

    /// The Dock's menu, which the app delegate hands back to the system.
    func dockMenu() -> NSMenu { dock.dockMenu() }

    func toggleQuickPanel() { quick.toggle { run(.showApp) } }

    /// The key's other action: bring the app forward, or hide it when it already is.
    func toggleApp() { NSApp.isActive ? NSApp.hide(nil) : NSApp.activate() }

    func run(_ command: AmbientCommand) {
        let hub = app.hub
        switch command {
        case .showApp: NSApp.activate()
        case .newTerminal: hub.send(.newTerminal)
        case .newWindow: app.openNewWindow()
        case .openTab(let tab): app.show(terminal: Int(tab))
        case .reply(let tab, let text): app.session(holding: Int(tab))?.hub.terminals.existing(Int(tab))?.source.write(Data((text + "\r").utf8))
        case .jumpToWaiting: hub.goTo(.needsInput)
        case .openSettings: app.showSettings()
        case .checkForUpdates: app.aux.showUpdates()
        case .quit: NSApp.terminate(nil)
        case .openFolder, .runAction, .openProject:
            // The folder picker and projects come with the core.
            break
        }
    }

    /// Puts the surfaces up for the terminals as they are, again whenever they change.
    private func follow() {
        let terminals = withObservationTracking { app.ambientTerminals } onChange: { [weak self] in
            Task { @MainActor in self?.follow() }
        }
        listed = terminals
        pushStatus()
        dock.update(terminals: terminals, badgeMode: badgeMode)
    }

    private func pushStatus() {
        var snapshot = StatusSnapshot()
        snapshot.terminals = listed
        snapshot.icon = iconMode
        snapshot.animate = animatesIcon
        status.update(snapshot)
    }
}

extension AppController {
    /// Every terminal of every window as the ambient surfaces list them.
    var ambientTerminals: [AmbientTerminal] {
        sessions.flatMap { $0.hub.state.sections.flatMap(\.rows) }.map { row in
            let terminal = row.terminal
            return AmbientTerminal(
                session: Int64(terminal.id), name: terminal.title, state: terminal.activity.ambient,
                since: activityClock.since(terminal.id, terminal.activity), source: terminal.agent == nil ? .plain : .agent,
                who: terminal.agent?.name, message: terminal.message, command: row.runningCommand,
                folder: terminal.folder, branch: terminal.branch)
        }
    }
}

private extension Activity {
    var ambient: AmbientState {
        switch self {
        case .idle: .idle
        case .working: .working
        case .needsInput: .needsYou
        case .finished: .done
        case .failed: .failed
        case .exited: .idle
        }
    }
}
