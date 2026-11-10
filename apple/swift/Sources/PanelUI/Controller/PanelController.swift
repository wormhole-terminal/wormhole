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
    /// A fixed clock (Unix seconds) for screenshots and tests; nil follows the real one.
    public var now: Int?

    public init(
        host: any PanelHost, backends: PanelBackends = .mock(), notes: NotesModel = NotesModel(), tab: PanelTab = .changes
    ) {
        self.host = host
        self.tab = tab
        changes = ChangesStore(git: backends.git, highlighter: backends.syntax, host: host, preferences: preferences)
        history = HistoryStore(git: backends.git, highlighter: backends.syntax, preferences: preferences)
        self.notes = NotesStore(model: notes, host: host)
        files = FilesStore(backend: backends.files, highlighter: backends.syntax, host: host)
        project = ProjectStore(backend: backends.project, host: host)
        usage = UsageStore(backend: backends.usage)
        worktrees = WorktreeStore(backend: backends.worktrees, host: host)
    }

    /// The terminal's folder changed (it `cd`ed, or another terminal was selected): every tab follows it.
    public func follow(folder: String?) {
        guard folder != self.folder else { return }
        self.folder = folder
        changes.follow(folder)
        history.follow(folder)
        files.follow(folder)
        project.follow(folder)
        worktrees.follow(folder)
    }

    /// File History: the log limited to the commits that touched `path`.
    public func showFileHistory(_ path: String) {
        history.filter(by: [path])
        tab = .history
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
        switch tab {
        case .changes: return changes.current == nil ? nil : .changesDiff
        case .history: return history.current == nil ? nil : .historyDiff
        case .files: return files.viewing == nil ? nil : .file
        default: return nil
        }
    }
}

extension PanelController {
    /// Esc over a diff or file: the thing laid over the terminal closes.
    public func closeCover() {
        switch cover {
        case .changesDiff: changes.close()
        case .historyDiff: history.close()
        case .file: files.closeViewer()
        case nil: break
        }
    }
}

public enum PanelCover: Sendable { case changesDiff, historyDiff, file }
