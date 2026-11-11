// The Changes tab's state: the status of the terminal's folder, how the list is laid out, the file open in the diff
// and the review comments written on its lines. Git is read through the backend, off the main actor, and read
// again when its watch says something changed.

import Observation
import PanelModel

@MainActor @Observable
public final class ChangesStore {
    public enum Phase: Equatable { case loading, loaded, notRepository, failed(String) }

    public private(set) var files: [ChangedFile] = []
    public private(set) var phase = Phase.loading
    public private(set) var folder: String?
    public var compare: Rev?
    /// The branch the list compared with is gone and the list is against HEAD (an amber line over it).
    public private(set) var compareMissing: String?
    /// The last read failed but the list under it is the last good one: git may only have been busy for a moment.
    public private(set) var refreshError: String?
    /// The list's arrangement: tree and untracked grouping are the panel's preferences, the folds are this list's own.
    public var layout: ChangeRows {
        get {
            var rows = arrangement
            // A comparison is one list: untracked files are not in it (`untracked_apart`).
            (rows.tree, rows.groupUntracked) = (preferences.changesTree, preferences.groupUntracked && compare == nil)
            return rows
        }
        set {
            arrangement = newValue
            (preferences.changesTree, preferences.groupUntracked) = (newValue.tree, newValue.groupUntracked)
        }
    }
    private var arrangement = ChangeRows()
    public let preferences: PanelPreferences
    public private(set) var selection: ChangedFile.ID?
    /// The row the keys are on (`PanelWalking`), and the files marked for the menu (⌘-click, ⇧-click, ⇧ with the arrows).
    public var cursor: String?
    public internal(set) var marked: Set<String> = []
    @ObservationIgnored var anchor: String?
    public var review = Review()
    public let diff: DiffSession
    public private(set) var revs: [Rev] = []
    /// The comments could not be pasted (the program would run each line): they went to the clipboard instead.
    public var copiedInstead = false
    @ObservationIgnored private let git: any GitBackend
    @ObservationIgnored private weak var host: (any PanelHost)?
    @ObservationIgnored private var watch: Task<Void, Never>?

    public init(
        git: any GitBackend, highlighter: any SyntaxHighlighting, host: (any PanelHost)? = nil,
        preferences: PanelPreferences = PanelPreferences()
    ) {
        self.git = git
        self.host = host
        self.preferences = preferences
        diff = DiffSession(highlighter: highlighter, preferences: preferences)
    }

    /// The tab follows the terminal: a new folder starts again.
    public func follow(_ folder: String?) {
        guard folder != self.folder else { return }
        self.folder = folder
        files = []
        compare = nil
        (compareMissing, refreshError) = (nil, nil)
        close()
        watch?.cancel()
        guard let folder else { return }
        phase = .loading
        refresh()
        watch = Task { [git, weak self] in
            for await _ in git.changes(in: folder) { self?.refresh() }
        }
    }

    public func refresh() {
        guard let folder else { return }
        let (git, rev) = (git, compare)
        Task {
            guard await git.isRepository(folder) else { return setPhase(.notRepository, folder) }
            do {
                let result = try await git.changesResult(in: folder, comparedWith: rev)
                let list = result.files
                guard folder == self.folder, rev == compare else { return }
                files = list
                marked.formIntersection(list.map(\.path))
                (compareMissing, refreshError) = (result.compareMissing, nil)
                phase = .loaded
                keepOpenDiffInStep()
            } catch {
                let message = (error as? GitFailure)?.message ?? panelText("git-unreadable", "git can't be read.")
                guard folder == self.folder else { return }
                // A list already shown stays under a red line; with none, the problem takes its place.
                if phase == .loaded { refreshError = message } else { phase = .failed(message) }
            }
        }
    }

    private func setPhase(_ new: Phase, _ folder: String) { if folder == self.folder { phase = new } }

    public var order: [ChangedFile] { layout.order(files) }
    /// The open file: its entry in the list, or as it was last read once the list no longer has it (the diff stays, "No longer changed").
    public var current: ChangedFile? { files.first { $0.id == selection } ?? (selection == nil ? nil : diff.file) }
    public var position: (index: Int, count: Int)? { current.flatMap { file in order.firstIndex(of: file).map { ($0, order.count) } } }

    public func open(_ id: ChangedFile.ID?) {
        if id != nil { marked = [] }
        selection = id
        guard let folder, let file = current else { return diff.close() }
        diff.open(file, stamp: stamp(of: file)) { [git, rev = compare] in
            try await git.diff(of: file.path, in: folder, comparedWith: rev, loadAnyway: $0)
        }
    }

    /// The status was read again: an open diff follows its file. One the list no longer has stays with its badge; one whose entry
    /// or file on disk changed is read again in place (`refresh_open_diff`).
    private func keepOpenDiffInStep() {
        guard let open = selection else { return }
        guard let now = files.first(where: { $0.id == open }) else { return diff.markGone() }
        let stamp = stamp(of: now)
        guard diff.gone || now != diff.file || stamp != diff.stamp else { return }
        diff.reread(now, stamp: stamp)
    }

    /// The file's size and time as the working tree has them now; nil in a comparison (its diff is against a revision, not the disk).
    private func stamp(of file: ChangedFile) -> String? {
        guard compare == nil, let folder, let path = RepoFiles.resolve(root: folder, relative: file.path) else { return nil }
        return host?.stamp(of: path)
    }

    public func close() { open(nil) }

    public func step(_ delta: Int) {
        if let next = ListNavigation.move(in: order.map(\.id), from: selection, by: delta) { open(next) }
    }

    /// Compare with a branch, tag or commit; nil goes back to the working tree against HEAD.
    public func setCompare(_ rev: Rev?) {
        compare = rev
        close()
        refresh()
    }

    /// The branches the picker lists, asked for when it opens.
    public func loadRevs() {
        guard let folder else { return }
        Task { revs = (try? await git.revs(in: folder, forHistory: false)) ?? [] }
    }

    /// Comments are made only on a diff against the working tree, whose lines are the ones the agent edits.
    public var canReview: Bool { compare == nil }

    /// Pastes the review into the terminal's prompt and gives it the keys: the person reads it there and presses
    /// return. The comments are gone once pasted. A program that takes no paste as text would run each line, so the
    /// review is copied instead and the comments stay.
    public func pasteReview() {
        guard let message = review.message, let host else { return }
        if host.paste(message) {
            review.clear()
            close()
            host.focusTerminal()
        } else {
            host.copy(message)
            copiedInstead = true
        }
    }
}
