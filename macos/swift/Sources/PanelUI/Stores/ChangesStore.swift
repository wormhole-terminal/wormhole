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
    /// The repository's root, which the paths of `files` are relative to (the controller sets it).
    @ObservationIgnored var root: String?
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
    /// The picker's list is being read ("Loading branches…").
    public private(set) var revsLoading = false
    /// The commit the list is against, and the one the open diff was read against: a diff is read again when it moves.
    @ObservationIgnored private var baseRev: String?
    @ObservationIgnored private var openBase: String?
    /// The comments could not be pasted (the program would run each line): they went to the clipboard instead.
    public var copiedInstead = false
    @ObservationIgnored private let git: any GitBackend
    @ObservationIgnored private weak var host: (any PanelHost)?
    @ObservationIgnored private var watch: Task<Void, Never>?
    /// A diff is about to open here: the controller takes down what else covers the terminal.
    @ObservationIgnored var opened: () -> Void = {}

    public init(
        git: any GitBackend, highlighter: any SyntaxHighlighting, host: (any PanelHost)? = nil,
        preferences: PanelPreferences = PanelPreferences()
    ) {
        self.git = git
        self.host = host
        self.preferences = preferences
        diff = DiffSession(highlighter: highlighter, preferences: preferences)
    }

    /// What belongs to one terminal and waits for it while another is shown (GPUI's `ChangesOf`, keyed by session).
    private struct Kept {
        var compare: Rev?
        var arrangement = ChangeRows()
        var marked: Set<String> = []
        var review = Review()
    }
    @ObservationIgnored private var kept: [Int: Kept] = [:]

    /// The window's terminals now: what waited for one that is gone (closed, or moved to another window) goes with it.
    public func keep(terminals: Set<Int>) { kept = kept.filter { terminals.contains($0.key) } }
    /// The terminal the tab shows: the picker closes when it changes.
    public private(set) var terminal: Int?

    /// The tab follows the terminal: another terminal brings its own compare branch, folds, marks and comments and
    /// takes the open diff down; a new folder starts again.
    public func follow(_ folder: String?, terminal: Int? = nil) {
        let moved = terminal != self.terminal
        guard moved || folder != self.folder else { return }
        if moved {
            if let old = self.terminal { kept[old] = Kept(compare: compare, arrangement: arrangement, marked: marked, review: review) }
            let mine = terminal.flatMap { kept.removeValue(forKey: $0) } ?? Kept()
            (compare, arrangement, marked, review) = (mine.compare, mine.arrangement, mine.marked, mine.review)
            (self.terminal, cursor, anchor, copiedInstead) = (terminal, nil, nil, false)
        }
        // The compare branch is the terminal's and survives a `cd` (only `setCompare` changes it); a branch the new folder
        // lacks shows as `compareMissing`.
        close()
        // The same folder under another terminal: the list is the same repository's, read for this one's branch.
        guard folder != self.folder else { return refresh() }
        self.folder = folder
        files = []
        (compareMissing, refreshError) = (nil, nil)
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
                (compareMissing, refreshError, baseRev) = (result.compareMissing, nil, result.baseRev)
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
        if id != nil {
            marked = []
            opened()
        }
        selection = id
        guard let folder, let file = current else { return diff.close() }
        openBase = baseRev
        diff.open(file, stamp: stamp(of: file)) { [git, rev = compare] in
            try await git.diff(of: file.path, in: folder, comparedWith: rev, loadAnyway: $0)
        }
    }

    /// The status was read again: an open diff follows its file. One the list no longer has stays with its badge; one whose entry
    /// or file on disk changed is read again in place (`refresh_open_diff`), and so is one whose base moved (the merge-base of
    /// a compared branch, HEAD after a commit).
    private func keepOpenDiffInStep() {
        guard let open = selection else { return }
        guard let now = files.first(where: { $0.id == open }) else { return diff.markGone() }
        let stamp = stamp(of: now)
        guard diff.gone || now != diff.file || stamp != diff.stamp || baseRev != openBase else { return }
        openBase = baseRev
        diff.reread(now, stamp: stamp)
    }

    /// The file's size and time as the working tree has them now: every diff here has the working tree as its new side, a
    /// comparison with a branch too.
    private func stamp(of file: ChangedFile) -> String? {
        guard let folder, let path = RepoFiles.resolve(root: root ?? folder, relative: file.path) else { return nil }
        return host?.stamp(of: path)
    }

    public func close() { open(nil) }

    public func step(_ delta: Int) {
        // The header's arrows go round the list (`rem_euclid`); the list's own arrow keys stop at its ends.
        let ids = order.map(\.id)
        guard !ids.isEmpty else { return }
        let at = selection.flatMap { ids.firstIndex(of: $0) } ?? (delta > 0 ? -1 : 0)
        open(ids[((at + delta) % ids.count + ids.count) % ids.count])
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
        (revs, revsLoading) = ([], true)
        Task {
            let listed = (try? await git.revs(in: folder, forHistory: false)) ?? []
            guard folder == self.folder else { return }
            (revs, revsLoading) = (listed, false)
        }
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
