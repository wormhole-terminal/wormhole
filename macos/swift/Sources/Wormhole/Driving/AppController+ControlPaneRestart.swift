// text-file: a test driver's replies on the control folder, not interface text
// The control folder's `pane-restart`: Restart on the bar over the selected terminal once its shell has ended, pressed
// (`WindowHub.restart`, what the bar's button is wired to): a new program in the same place, in a new host when they are
// on. Refused while the terminal has no such bar (its program runs): `exited-words` says when it has.
// Replies `pane-restart <terminal>`, or `error: …`.

import Foundation

extension AppController {
    /// False when `command` is not `pane-restart`.
    func runPaneRestart(command: String) -> Bool {
        guard command == "pane-restart" else { return false }
        guard let id = hub.state.selected, hub.exitedWords != nil else {
            reply("error: the selected terminal's shell has not ended: its bar offers no Restart")
            return true
        }
        hub.restart(id)
        reply("pane-restart \(id)")
        return true
    }
}
