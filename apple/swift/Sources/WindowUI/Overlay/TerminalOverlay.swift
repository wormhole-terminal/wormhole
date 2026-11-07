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
    /// The shell ended because its session host went away with it (the exited bar says so).
    var hostLost = false
    /// "Wait" was pressed on the bar for a host that does not answer: it stays away until the host answers again.
    var waiting = false
    /// What the bar says of the session host; a program's clipboard question goes over it and gives it back.
    private var hostBar: PaneBar?
    /// Why ending the host from its bar failed, until something else happens.
    var endFailure: String?
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

    func exited(code: Int32?, hostLost: Bool) {
        exitCode = code.map(Int.init)
        self.hostLost = hostLost
        bars.exited = true
    }

    /// The bar for a session host's trouble, below a program's question (which stays until it is answered).
    func showHost(_ problem: HostProblem?) {
        if problem != .notResponding { waiting = false }
        switch problem {
        case .unavailable(let why)?: hostBar = .hostUnavailable(HostUnavailableKind(why), endFailure: endFailure)
        case .notResponding? where !waiting: hostBar = .hostNotResponding(endFailure: endFailure)
        default: hostBar = nil
        }
        askEnded()
    }

    /// No program's question is up (or none now): the bar is the host's trouble, if there is any.
    func askEnded() {
        if case .ask? = bars.bar { return }
        bars.bar = hostBar
    }
}

extension HostUnavailableKind {
    init(_ why: HostProblem.Why) {
        switch why {
        case .refused: self = .refused
        case .tooOld: self = .tooOld
        case .tooNew: self = .tooNew
        case .busy: self = .busy
        case .timedOut: self = .timedOut
        case .unreachable: self = .unreachable
        case .failed: self = .failed
        }
    }
}

/// `PaneBarActions` over closures: the session host's buttons, restarting and closing are the window's; the clipboard
/// answer goes to the core (`TerminalOverlay+Clipboard`).
@MainActor
final class BarActions: PaneBarActions {
    var onRestart: () -> Void = {}
    var onClose: () -> Void = {}
    var onReconnect: () -> Void = {}
    var onEndHost: (Bool) -> Void = { _ in }
    var onWait: () -> Void = {}

    /// The person's choice: the core answers the program (`TerminalOverlay+Clipboard`).
    var onAnswer: (ClipboardChoice) -> Void = { _ in }
    func answerClipboard(_ choice: ClipboardChoice) { onAnswer(choice) }
    func tryAttachAgain() { onReconnect() }
    func openNewTerminalHere() { onRestart() }
    func endHost(hard: Bool) { onEndHost(hard) }
    func waitForHost() { onWait() }
    func restart() { onRestart() }
    func closeTerminal() { onClose() }
    func dismissHostNotice() {}
}
