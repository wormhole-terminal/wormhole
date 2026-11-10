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
            (rows.tree, rows.groupUntracked) = (preferences.changesTree, preferences.groupUntracked)
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
                (compareMissing, refreshError) = (result.compareMissing, nil)
                phase = .loaded
                if let open = selection, !list.contains(where: { $0.id == open }) { close() }
            } catch {
                let message = (error as? GitFailure)?.message ?? "git can't be read."
                guard folder == self.folder else { return }
                // A list already shown stays under a red line; with none, the problem takes its place.
                if phase == .loaded { refreshError = message } else { phase = .failed(message) }
            }
        }
    }

    private func setPhase(_ new: Phase, _ folder: String) { if folder == self.folder { phase = new } }

    public var order: [ChangedFile] { layout.order(files) }
    public var current: ChangedFile? { files.first { $0.id == selection } }
    public var position: (index: Int, count: Int)? { current.flatMap { file in order.firstIndex(of: file).map { ($0, order.count) } } }

    public func open(_ id: ChangedFile.ID?) {
        selection = id
        guard let folder, let file = current else { return diff.close() }
        let (git, rev) = (git, compare)
        diff.open(file) { try await git.diff(of: file.path, in: folder, comparedWith: rev) }
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
