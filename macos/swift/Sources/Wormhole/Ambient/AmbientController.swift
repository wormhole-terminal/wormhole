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
    private var listedActions: [StatusSnapshot.Action] = []
    private var listedRecent: [StatusSnapshot.Recent] = []
    var animatesIcon = false { didSet { if animatesIcon != oldValue { pushStatus() } } }
    private let quick: QuickPanelController
    unowned let app: AppController

    init?(app: AppController) {
        guard app.policy.touchesSystem else { return nil }
        self.app = app
        status = StatusItemController(policy: app.policy)
        dock = DockController(policy: app.policy)
        quick = QuickPanelController { [weak app] in (app?.ambientTerminals ?? [], app?.ambientItems ?? []) }
        let route: (MenuCommand) -> Void = { [weak app] in app?.router.dispatch($0) }
        status.dispatcher.handler = route
        dock.dispatcher.handler = route
        quick.model.send = { [weak self] in self?.run($0) }
        follow()
    }

    /// The Dock's menu, which the app delegate hands back to the system.
    func dockMenu() -> NSMenu { dock.dockMenu() }

    func toggleQuickPanel() { quick.toggle { run(.showApp) } }

    /// The global key was pressed (`HotkeyController`): what Settings says it does.
    func hotkeyPressed(_ action: HotkeyController.Action) {
        switch action {
        case .quickPanel: toggleQuickPanel()
        case .toggleApp: toggleApp()
        }
    }

    /// The key's other action: bring the app forward, or hide it when it already is.
    func toggleApp() { NSApp.isActive ? NSApp.hide(nil) : NSApp.activate() }

    func run(_ command: AmbientCommand) { app.run(ambient: command) }

    /// Puts the surfaces up for the terminals as they are, again whenever they change.
    private func follow() {
        let (terminals, actions, recent) = withObservationTracking { (app.ambientTerminals, app.ambientActions, app.ambientRecent) } onChange: { [weak self] in
            Task { @MainActor in self?.follow() }
        }
        (listed, listedActions, listedRecent) = (terminals, actions, recent)
        pushStatus()
        dock.update(terminals: terminals, badgeMode: badgeMode)
    }

    /// The status item's menu as things are now; also when a key it shows changed (`apply`, `bindJumpKey`), which no
    /// terminal's change carries.
    func pushStatus() {
        var snapshot = StatusSnapshot()
        snapshot.terminals = listed
        (snapshot.actions, snapshot.recent) = (listedActions, listedRecent)
        // The two keys the menu shows beside Show Wormhole and Jump to Next Waiting (`menubar` `hotkey`, `jump_key`).
        snapshot.hotkey = app.settingsModel.value("hotkey").string.flatMap { try? KeySpec(parsing: $0) }
        snapshot.jumpKey = Keymap.shared.key("goToWaiting")
        snapshot.icon = iconMode
        snapshot.animate = animatesIcon
        status.update(snapshot)
    }
}

extension AppController {
    /// The actions of the trusted project of every open terminal of the profile shown (what the status item lists under Projects).
    var ambientActions: [StatusSnapshot.Action] {
        var roots = Set<String>()
        return shown.compactMap { $0.hub.panel.project.state }.filter { roots.insert($0.root).inserted }
            .flatMap { state in state.actions.map { .init(project: state.name, root: state.root, id: $0.id, title: $0.title) } }
    }

    /// The projects worked in lately, for the status item's list.
    var ambientRecent: [StatusSnapshot.Recent] { recents.roots(of: windowsHost.profile).map { .init(name: ($0 as NSString).lastPathComponent, path: $0) } }

    /// What the quick panel's Do section offers: a new terminal where the one in front is, Open folder…, the project's actions.
    var ambientItems: [QuickPanelDo] {
        let folder = hub.state.all.first { $0.id == hub.state.selected }?.folder.map { HomeFolder.expanding($0) }
        return [.newTerminal(cwd: folder), .openFolder] + ambientActions.map(QuickPanelDo.action)
    }

    /// Every terminal of every window as the ambient surfaces list them, those in folded groups too (GPUI's `tab_facts` walks
    /// every session).
    var ambientTerminals: [AmbientTerminal] {
        let terminals: [AmbientTerminal] = sessions.flatMap { session in session.hub.state.all.map { ambient($0, running: $0.runningCommand) } }
        #if WITH_UNIFFI
            return namedByReports(terminals)
        #else
            return terminals
        #endif
    }

    /// What the surfaces call a terminal's folder (core's `notify::Terminal.folder`): its name, "~" being the home folder's.
    static func folderName(_ path: String) -> String {
        let full = HomeFolder.expanding(path)
        return full == "/" ? "/" : (full as NSString).lastPathComponent
    }

    func ambient(_ terminal: TerminalSummary, running: String?) -> AmbientTerminal {
        AmbientTerminal(
            session: Int64(terminal.id), name: terminal.title, state: terminal.activity.ambient,
            since: activityClock.since(terminal.id, terminal.activity), source: terminal.agent == nil ? .plain : .agent,
            who: terminal.agent?.name, message: terminal.message, command: running,
            folder: terminal.folder.map(Self.folderName), branch: terminal.branch)
    }
}

extension Activity {
    fileprivate var ambient: AmbientState {
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
