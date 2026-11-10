// Every menu, palette and key action by its keymap id, onto the window's own verbs: one table, so a key, a menu
// item and the status item reach the same code. An action with no entry has no Swift implementation yet and is
// greyed in the menus (`MenuStateMaker`), never left out.

import AppKit
import PlatformModel
import PlatformUI
import WindowUI

@MainActor
final class ActionRouter {
    typealias Handler = @MainActor () -> Void
    private(set) var table: [ActionID: Handler] = [:]
    unowned let app: AppController

    init(app: AppController) {
        self.app = app
        table = terminalActions().merging(paneActions(), uniquingKeysWith: { first, _ in first })
            .merging(viewActions(), uniquingKeysWith: { first, _ in first })
            .merging(findActions(), uniquingKeysWith: { first, _ in first })
            .merging(outputActions(), uniquingKeysWith: { first, _ in first })
            .merging(appActions(), uniquingKeysWith: { first, _ in first })
            .merging(editActions(), uniquingKeysWith: { first, _ in first })
    }

    /// What the menus may enable: the table, and what AppKit answers itself (hide, minimise, the editing keys).
    var implemented: Set<ActionID> { Set(table.keys).union(StandardActions.ids).union(Self.viewed) }

    func run(_ id: ActionID) {
        if let handler = table[id] { handler() }
    }

    /// A choice from any menu: the bar, a context menu, the status item or the Dock.
    func dispatch(_ command: MenuCommand) {
        switch command {
        case .action(let id): run(id)
        case .ambient(let command): app.ambient?.run(command)
        case .context(let action): app.runContext(action)
        case .switchProfile(let index): app.switchProfile(index: index)
        case .openInProfile(let index): app.openWindow(inProfileIndex: index)
        case .moveToProfile(let index): app.moveFrontWindow(toProfileIndex: index)
        case .openRecent, .runProjectAction, .newTerminalWith:
            // Recent folders, project actions and shells come with the core.
            break
        }
    }
}

extension StandardActions {
    /// The ids `StandardActions.selectors` answer.
    static let ids = Set(selectors.keys.compactMap { command -> ActionID? in
        if case .action(let id) = command { id } else { nil }
    })
}
