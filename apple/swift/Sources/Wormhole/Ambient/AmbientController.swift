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
    private let status: StatusItemController
    private let dock: DockController
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

    func run(_ command: AmbientCommand) {
        let hub = app.hub
        switch command {
        case .showApp: NSApp.activate()
        case .newTerminal: hub.send(.newTerminal)
        case .openTab(let tab): hub.send(.select(Int(tab)))
        case .reply(let tab, let text): hub.terminals.existing(Int(tab))?.source.write(Data((text + "\r").utf8))
        case .jumpToWaiting: hub.goTo(.needsInput)
        case .openSettings: app.showSettings()
        case .checkForUpdates: app.aux.showUpdates()
        case .quit: NSApp.terminate(nil)
        case .newWindow, .openFolder, .runAction, .openProject:
            // Several windows, the folder picker and projects come with the core.
            break
        }
    }

    /// Puts the surfaces up for the terminals as they are, again whenever they change.
    private func follow() {
        let terminals = withObservationTracking { app.ambientTerminals } onChange: { [weak self] in
            Task { @MainActor in self?.follow() }
        }
        var snapshot = StatusSnapshot()
        snapshot.terminals = terminals
        status.update(snapshot)
        dock.update(terminals: terminals, badgeMode: .needsYou)
    }
}

extension AppController {
    /// Every terminal of the window as the ambient surfaces list them.
    var ambientTerminals: [AmbientTerminal] {
        hub.state.terminals.map { terminal in
            AmbientTerminal(
                session: Int64(terminal.id), name: terminal.title, state: terminal.activity.ambient,
                since: terminal.activitySince ?? Date(), source: terminal.agent == nil ? .plain : .agent,
                who: terminal.agent?.name, message: terminal.message, command: terminal.runningCommand,
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
        }
    }
}
