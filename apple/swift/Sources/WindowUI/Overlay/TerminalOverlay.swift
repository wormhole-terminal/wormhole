// What a terminal carries over its grid, kept with the terminal and not the pane: its find bar, the size chip and
// the bars along the bottom (a program's question, a host's trouble, the shell that ended). One per terminal, made
// with its view, so a terminal moved to another pane or hidden behind a diff keeps its search and its bar.

import Observation
import TerminalMetal
import TerminalUI
import WorkspaceModel

@MainActor @Observable
final class TerminalOverlay {
    let find: FindModel
    let size = SizeReadoutModel()
    var bars = PaneOverlayState()
    var exitCode: Int?
    /// The bars' buttons, over the window's own actions.
    @ObservationIgnored let actions: BarActions

    init(view: MetalTerminalView, actions: BarActions) {
        find = FindModel(searcher: ScreenSearching(view: view))
        self.actions = actions
    }

    func exited(code: Int32?) {
        exitCode = code.map(Int.init)
        bars.exited = true
    }
}

/// `PaneBarActions` over closures: what needs the Rust session host (attaching again, ending it, the clipboard
/// question) has nothing to do yet and does nothing; restarting and closing are the window's.
@MainActor
final class BarActions: PaneBarActions {
    var onRestart: () -> Void = {}
    var onClose: () -> Void = {}

    func answerClipboard(_ choice: ClipboardChoice) {}
    func tryAttachAgain() {}
    func openNewTerminalHere() {}
    func endHost(hard: Bool) {}
    func waitForHost() {}
    func restart() { onRestart() }
    func closeTerminal() { onClose() }
    func dismissHostNotice() {}
}
