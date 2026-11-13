// The right panel's state as one object the window owns per terminal: the tab shown, the store of each tab, and the
// terminal's folder they all follow. Views take the store they draw; this is what composes them and answers for
// the strip's count badges. Nothing here draws.

import Observation
import PanelModel

@MainActor @Observable
public final class PanelController {
    public var tab: PanelTab
    public let changes: ChangesStore
    public let history: HistoryStore
    public let notes: NotesStore
    public let files: FilesStore
    public let project: ProjectStore
    public let usage: UsageStore
    public let worktrees: WorktreeStore
    public let host: any PanelHost
    /// What Settings → Git says, shared by the tabs.
    public let preferences = PanelPreferences()
    public let keyboard = PanelKeyboard()
    public private(set) var folder: String?
    @ObservationIgnored private var terminal: Int?
    @ObservationIgnored private var root: String?
    @ObservationIgnored private var roots: [String: String] = [:]
    @ObservationIgnored private let git: any GitBackend
    /// A fixed clock (Unix seconds) for screenshots and tests; nil follows the real one.
    public var now: Int?

    public init(
        host: any PanelHost, backends: PanelBackends = .mock(), notes: NotesModel = NotesModel(), tab: PanelTab = .changes
    ) {
        self.host = host
        self.tab = tab
        git = backends.git
        changes = ChangesStore(git: backends.git, highlighter: backends.syntax, host: host, preferences: preferences)
        history = HistoryStore(git: backends.git, highlighter: backends.syntax, preferences: preferences)
        self.notes = NotesStore(model: notes, host: host)
        files = FilesStore(backend: backends.files, highlighter: backends.syntax, host: host)
        project = ProjectStore(backend: backends.project, host: host)
        usage = UsageStore(backend: backends.usage)
        worktrees = WorktreeStore(backend: backends.worktrees, host: host)
        changes.opened = { [unowned self] in
            history.close()
            files.closeViewer()
        }
        history.opened = { [unowned self] in
            changes.close()
            files.closeViewer()
        }
        files.opened = { [unowned self] in
            changes.close()
            history.close()
        }
    }

    /// The terminal's folder changed (it `cd`ed, or another terminal was selected): every tab follows it.
    /// `terminal` is the selected terminal's id: what a tab keeps per terminal waits under it. `root` is the folder's
    /// repository root, else the folder: the band's folder, which the Files tree and the project's colour are of.
    private func settle(folder: String?, terminal: Int?, root: String?) {
        guard folder != self.folder || terminal != self.terminal || root != self.root else { return }
        (self.folder, self.terminal, self.root) = (folder, terminal, root)
        changes.follow(folder, terminal: terminal)
        history.follow(folder, terminal: terminal)
        files.follow(root ?? folder, terminal: terminal)
        project.follow(folder, band: root ?? folder)
        worktrees.follow(folder)
    }

    /// The window's terminals now. What a tab kept for a terminal that is gone (closed, or moved to another window) is
    /// dropped, as the GPUI app's features do in `closed` and `left`, and so is each folder's root no terminal is in
    /// any more (`folders`: the terminals' folders), which the Project tab's colour and Create are for.
    public func keep(terminals: Set<Int>, folders: Set<String>) {
        changes.keep(terminals: terminals)
        history.keep(terminals: terminals)
        files.keep(terminals: terminals)
        roots = roots.filter { folders.contains($0.key) }
    }

    /// The selected terminal and its folder. The band's folder is asked of git once per folder and remembered, so
    /// coming back to a terminal finds its tree at once.
    public func follow(folder: String?, terminal: Int? = nil) {
        settle(folder: folder, terminal: terminal, root: folder.flatMap { roots[$0] })
        guard let folder, roots[folder] == nil else { return }
        Task { [git] in
            guard let root = await git.root(of: folder) else { return }
            roots[folder] = root
            if folder == self.folder { settle(folder: folder, terminal: self.terminal, root: root) }
        }
    }

    
    /// File History: the log limited to the commits that touched these paths.
    public func showFileHistory(_ paths: [String]) {
        history.filter(by: paths)
        tab = .history
        // The window's strip is the window's: it is told, or it would still mark the tab this came from.
        host.show(tab: .history)
    }

    /// The count on a tab of the strip: the changed files (Changes is the only tab with one, as in the GPUI app).
    public func badge(for tab: PanelTab) -> Int? {
        tab == .changes && !changes.files.isEmpty ? changes.files.count : nil
    }

    /// Where an opened diff or file is drawn (see `DiffPresentation`).
    public var presentation = DiffPresentation.panel

    /// What the window lays over the terminal in its place, when the presentation says so: the diff of the file open
    /// in Changes or History, or the file open in Files.
    public var cover: PanelCover? {
        guard presentation == .terminalArea else { return nil }
        // One cover per window whatever tab shows: opening one thing takes the other down (`opened` below).
        if changes.current != nil { return .changesDiff }
        if history.current != nil { return .historyDiff }
        return files.viewing == nil ? nil : .file
    }
}

extension PanelController {
    /// Esc over a diff or file: the thing laid over the terminal closes.
    public func closeCover() {
        switch cover {
        case .changesDiff: changes.close()
        case .historyDiff: history.close()
        case .file: files.closeViewer()
        case nil: return
        }
        // The terminal it covered has the keys again (`close_diff`, the viewer's `close`).
        host.focusTerminal()
    }
}

public enum PanelCover: Sendable { case changesDiff, historyDiff, file }
