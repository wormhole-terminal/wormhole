// The History tab's state: the log of a branch (or the stashes), the commit opened and its details, and the file of
// that commit whose diff is open. A log can be limited to the commits that touched some paths ("File History").

import Observation
import PanelModel

@MainActor @Observable
public final class HistoryStore {
    public enum Phase: Equatable { case loading, loaded, notRepository, failed(String) }

    public private(set) var model = HistoryModel()
    public private(set) var phase = Phase.loading
    public private(set) var folder: String?
    public private(set) var rev: Rev?
    public private(set) var paths: [String] = []
    public private(set) var expanded: Commit.ID?
    /// The opened commit's block as the core made it, once read (the log's rows carry the subject only).
    public private(set) var details: CommitDetails?
    /// Why the details could not be read.
    public private(set) var detailsError: String?
    public private(set) var selection: ChangedFile.ID?
    /// Counts up with every ↩ on the open commit's details: the block answers with its menu.
    public internal(set) var detailsMenus = 0
    /// The row the keys are on (`PanelWalking`).
    public var cursor: String?
    public private(set) var revs: [Rev] = []
    public let diff: DiffSession
    @ObservationIgnored private let git: any GitBackend
    @ObservationIgnored private var watch: Task<Void, Never>?
    /// A diff is about to open here: the controller takes down what else covers the terminal.
    @ObservationIgnored var opened: () -> Void = {}

    public init(git: any GitBackend, highlighter: any SyntaxHighlighting, preferences: PanelPreferences = PanelPreferences()) {
        self.git = git
        diff = DiffSession(highlighter: highlighter, preferences: preferences)
    }

    /// What belongs to one terminal and waits for it while another is shown (GPUI's `HistoryState` per session).
    private struct Kept {
        var rev: Rev?
        var paths: [String] = []
        var expanded: Commit.ID?
    }
    @ObservationIgnored private var kept: [Int: Kept] = [:]
    @ObservationIgnored private var terminal: Int?

    /// Another terminal brings its own picked rev, path filter and open commit; a `cd` keeps the picked rev.
    public func follow(_ folder: String?, terminal: Int? = nil) {
        let moved = terminal != self.terminal
        guard moved || folder != self.folder else { return }
        close()
        if moved {
            if let old = self.terminal { kept[old] = Kept(rev: rev, paths: paths, expanded: expanded) }
            let mine = terminal.flatMap { kept.removeValue(forKey: $0) } ?? Kept()
            (rev, paths, expanded, self.terminal, cursor) = (mine.rev, mine.paths, mine.expanded, terminal, nil)
        } else {
            (paths, expanded) = ([], nil)
        }
        (details, detailsError) = (nil, nil)
        guard folder != self.folder else {
            refresh()
            return readDetails()
        }
        self.folder = folder
        model = HistoryModel()
        watch?.cancel()
        guard let folder else { return }
        phase = .loading
        refresh()
        readDetails()
        watch = Task { [git, weak self] in
            for await _ in git.changes(in: folder) { self?.refresh() }
        }
    }

    public func refresh() {
        guard let folder else { return }
        let (git, rev, paths) = (git, rev, paths)
        Task {
            guard await git.isRepository(folder) else { if folder == self.folder { phase = .notRepository }; return }
            do {
                let log = try await git.log(in: folder, of: rev, paths: paths)
                guard folder == self.folder, rev == self.rev, paths == self.paths else { return }
                let moved = model != log
                model = log
                phase = .loaded
                // A branch or tag may have moved onto the open commit: its block is read again, the old one staying meanwhile.
                if moved, details != nil { readDetails() }
            } catch {
                if folder == self.folder { phase = .failed((error as? GitFailure)?.message ?? panelText("history-unreadable", "The log can't be read.")) }
            }
        }
    }

    /// Opens a commit, or closes it when it is the one open.
    public func toggle(_ sha: Commit.ID) {
        close()
        details = nil
        guard expanded != sha else { return (expanded, detailsError) = (nil, nil) }
        (expanded, detailsError) = (sha, nil)
        readDetails()
    }

    private func readDetails() {
        guard let folder, let sha = expanded else { return }
        let git = git
        Task {
            do {
                let full = try await git.details(of: sha, in: folder)
                if expanded == sha, folder == self.folder { details = full }
            } catch {
                if expanded == sha { detailsError = (error as? GitFailure)?.message ?? panelText("history-details-unreadable", "The details can't be read.") }
            }
        }
    }

    /// What `what` of `commit` puts on the clipboard: read from the log row, or from the details, which a commit that
    /// was never opened has read first. Nil when git cannot say.
    public func text(of what: CommitCopy, for commit: Commit) async -> String? {
        let known = details?.sha == commit.sha ? details : nil
        if let text = what.text(of: commit, details: known) { return text }
        guard let folder, let read = try? await git.details(of: commit.sha, in: folder) else { return nil }
        return what.text(of: commit, details: read)
    }

    /// What the chip says of copying `what` of `commit`: its message is cut when the details were read cut.
    public func copied(_ what: CommitCopy, for commit: Commit) -> PanelCopied {
        what.copied(cut: details?.sha == commit.sha && details?.tooLong != nil)
    }

    /// The open commit's message for lines of about `lineChars` characters, and whether it leaves anything out.
    public func compact(lineChars: Int) -> CommitDetails.Compact? {
        guard let folder, let sha = expanded else { return nil }
        return git.compactMessage(of: sha, in: folder, lineChars: lineChars)
    }

    /// Opens the commit `sha` names (a parent's link), if the log has it; one that is open stays open.
    public func show(_ sha: String) {
        guard let commit = model.commits.first(where: { $0.sha.hasPrefix(sha.prefix(7)) }), expanded != commit.id else { return }
        toggle(commit.id)
        // The list scrolls to the row the keys are on.
        cursor = "commit-" + commit.id
    }

    public func pick(_ rev: Rev?) {
        self.rev = rev
        (expanded, details, detailsError) = (nil, nil, nil)
        close()
        refresh()
    }

    public func filter(by paths: [String]) {
        self.paths = paths
        // The log is another list now: the open commit folds (`show_paths`).
        (expanded, details, detailsError) = (nil, nil, nil)
        close()
        refresh()
    }

    public var files: [ChangedFile] { details?.files ?? [] }
    public var current: ChangedFile? { files.first { $0.id == selection } }
    public var position: (index: Int, count: Int)? { current.flatMap { file in files.firstIndex(of: file).map { ($0, files.count) } } }

    /// Opens one file of the open commit in the diff.
    public func open(_ id: ChangedFile.ID?) {
        if id != nil { opened() }
        selection = id
        guard let folder, let sha = expanded, let file = current else { return diff.close() }
        let git = git
        diff.open(file) { try await git.diff(of: file.path, inCommit: sha, folder: folder, loadAnyway: $0) }
    }

    public func close() { open(nil) }

    public func step(_ delta: Int) {
        // Round the commit's files, as the Changes diff's arrows go.
        let ids = files.map(\.id)
        guard !ids.isEmpty else { return }
        let at = selection.flatMap { ids.firstIndex(of: $0) } ?? (delta > 0 ? -1 : 0)
        open(ids[((at + delta) % ids.count + ids.count) % ids.count])
    }

    public func loadRevs() {
        guard let folder else { return }
        Task { revs = (try? await git.revs(in: folder, forHistory: true)) ?? [] }
    }
}
