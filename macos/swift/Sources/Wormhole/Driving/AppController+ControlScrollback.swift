// text-file: a test driver's replies on the control folder, not interface text
// The control folder's `scrollback [terminal]`: how many lines of history that terminal's view was last given, how far back it
// is scrolled and its rows, from the last frame's header (`MetalTerminalView.scrollbackReadout`, what its scroll bar is drawn
// from). Replies `scrollback history=<lines> offset=<lines back> rows=<n>`, or `scrollback none` before a first frame.

import TerminalMetal

extension AppController {
    /// False when `command` is not `scrollback`.
    func runScrollback(command: String, argument: String) -> Bool {
        guard command == "scrollback" else { return false }
        let view = Int(argument.trimmingCharacters(in: .whitespaces)).flatMap { hub.terminals.existing($0) } ?? selectedView
        reply(view?.scrollbackReadout ?? "error: no terminal is shown")
        return true
    }
}
