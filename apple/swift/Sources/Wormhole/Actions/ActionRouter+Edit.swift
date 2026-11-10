// The terminal's editing verbs and the few window verbs the menus reach by id: Clear Buffer, a new note, Hide from Agents,
// a terminal in a new worktree, and the intros played again (Help's trial entries). The line editor's keys (⌘← ⌘→ ⌘⌫), the
// notes' and Studio's own keys and the profile actions that need a profile are the views' (`viewed`).

import AppKit
import AuxModel
import PlatformModel
import WindowUI
import WorkspaceModel

extension ActionRouter {
    func editActions() -> [ActionID: Handler] {
        var hub: WindowHub { app.hub }
        var table: [ActionID: Handler] = [
            "clearBuffer": { [unowned self] in app.selectedView?.clearBuffer(nil) },
            "newNote": {
                hub.send(.showPanelTab(.notes))
                hub.panel.notes.add(note: "")
            },
            "hideFromAgents": {
                guard let terminal = hub.state.all.first(where: { $0.id == hub.state.selected }) else { return }
                hub.send(.setAgentHidden(terminal.id, !terminal.isAgentHidden))
            },
            // A worktree needs a repository: the menu greys this where the terminal's folder is not in one.
            "newTerminalInWorktree": {
                hub.send(.showPanelTab(.changes))
                hub.panel.worktrees.ask()
            },
            "insertFolderPath": { [unowned self] in app.folderPicking.insertPath() },
        ]
        for style in IntroStyle.onTrial {
            table[ActionID(rawValue: "playIntro" + style.rawValue.capitalized)] = { [unowned self] in app.aux.playIntro(style) }
        }
        for handover in Handover.allCases {
            table[ActionID(rawValue: "playWelcome" + handover.rawValue.capitalized)] = { [unowned self] in
                app.aux.playIntro(.voyage, handover: handover)
            }
        }
        return table
    }

    /// Actions the keymap lists that a view carries out itself, as in the GPUI app (`actions::gpui_action` has none for
    /// them): the terminal's line editor keys, an editor's ⌥⌘↑/↓, the Theme Studio's save and duplicate, and the profile
    /// actions, which act on the profile a menu item or palette row names (`dispatch`).
    static let viewed: Set<ActionID> = [
        "lineStart", "lineEnd", "deleteLine", "moveItemUp", "moveItemDown", "saveTheme", "duplicateTheme",
        "switchProfile", "openWindowInProfile", "moveWindowToProfile",
    ]
}
