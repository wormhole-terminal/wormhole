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
    /// The command blocks of the terminal's scrollback (`TerminalOverlay+Blocks`): marks, hover, jumps and actions.
    let blocks: BlockController
    let minimap: CoreMinimap
    /// Bumped after every frame that rebuilt rows: what is drawn from the terminal beside the grid reads it.
    private(set) var revision = 0
    @ObservationIgnored private(set) weak var view: MetalTerminalView?
    let size = SizeReadoutModel()
    var bars = PaneOverlayState()
    var exitCode: Int?
    /// The bars' buttons, over the window's own actions.
    @ObservationIgnored let actions: BarActions

    init(view: MetalTerminalView, actions: BarActions, notices: NoticeCenter) {
        let find = FindModel(searcher: ScreenSearching(view: view))
        let blocks = BlockController(
            source: CoreBlocks(
                terminal: { [weak view] in view?.source.scrollback },
                palette: { [weak view] in view?.renderer.palette.slots.map(Self.rgb) ?? [] }),
            say: { notices.post($0) }, announce: Self.announce)
        (self.find, self.blocks, self.view) = (find, blocks, view)
        minimap = CoreMinimap(
            terminal: { [weak view] in view?.source.scrollback }, palette: { [weak view] in view?.renderer.palette ?? .spike })
        self.actions = actions
        view.onPointerRow = { [weak blocks] in blocks?.hover(row: $0) }
        wireClipboard(view: view, notices: notices)
        view.onContentChanged = { [weak self] in self?.contentChanged() }
    }

    /// The terminal drew rows: the views beside the grid read again, and a search that the output made stale runs again.
    private func contentChanged() {
        revision &+= 1
        blocks.noteView()
        find.outputChanged()
    }

    func exited(code: Int32?) {
        exitCode = code.map(Int.init)
        bars.exited = true
    }
}

/// `PaneBarActions` over closures: what needs the Rust session host (attaching again, ending it)
/// has nothing to do yet and does nothing; restarting and closing are the window's.
@MainActor
final class BarActions: PaneBarActions {
    var onRestart: () -> Void = {}
    var onClose: () -> Void = {}

    /// The person's choice: the core answers the program (`TerminalOverlay+Clipboard`).
    var onAnswer: (ClipboardChoice) -> Void = { _ in }
    func answerClipboard(_ choice: ClipboardChoice) { onAnswer(choice) }
    func tryAttachAgain() {}
    func openNewTerminalHere() {}
    func endHost(hard: Bool) {}
    func waitForHost() {}
    func restart() { onRestart() }
    func closeTerminal() { onClose() }
    func dismissHostNotice() {}
}
