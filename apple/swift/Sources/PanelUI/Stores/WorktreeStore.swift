// A worktree per terminal, as the panel hosts it: the sheet that asks for a branch before making one, and the
// confirmation before removing the one Wormhole made for this terminal. Writes only on the person's action, and
// the backend refuses in a project that isn't trusted.

import Observation
import PanelModel

@MainActor @Observable
public final class WorktreeStore {
    public private(set) var folder: String?
    /// The new-worktree sheet is up.
    public var asking = false { didSet { if !asking { askedFolder = nil } } }
    /// The repository a command-line request named, for as long as its sheet is up; else the terminal's folder.
    @ObservationIgnored private var askedFolder: String?
    public var branch = "" { didSet { if branch != oldValue { replan() } } }
    public private(set) var plan: WorktreePlan?
    public private(set) var error: String?
    public private(set) var busy = false
    /// The worktree Wormhole made that the terminal is in, which can be removed.
    public private(set) var made: MadeWorktree?
    public var confirmingRemoval = false
    /// Whether the folder has uncommitted changes as last read: a worktree with some stays.
    public var dirty = false
    @ObservationIgnored private let backend: any WorktreeBackend
    @ObservationIgnored private weak var host: (any PanelHost)?
    @ObservationIgnored private var planning: Task<Void, Never>?

    public init(backend: any WorktreeBackend, host: (any PanelHost)? = nil) {
        self.backend = backend
        self.host = host
    }

    public func follow(_ folder: String?) {
        self.folder = folder
        made = nil
        guard let folder else { return }
        Task { made = await backend.made(at: folder) }
    }

    /// The sheet's footnote under the branch field: the folder it would make, or why it can't.
    public var footnote: String? {
        switch plan {
        case let .ready(_, folder): "Folder: \(folder)"
        case let .refused(why): why
        case nil: nil
        }
    }

    public var canCreate: Bool { if case .ready = plan, !busy { true } else { false } }

    /// The sheet, empty or with `branch` filled in, for the terminal's folder or for `repo` (`wh new-terminal
    /// --worktree <branch>` names the folder it was run for). The backend may refuse (an untrusted project): that is said, not asked.
    public func ask(branch preset: String = "", in repo: String? = nil) {
        guard let target = repo ?? folder else { return }
        // Before the branch, which plans for the folder it finds.
        askedFolder = repo
        plan = nil
        error = nil
        branch = ""
        branch = preset
        Task {
            if let problem = await backend.problem(making: target) {
                askedFolder = nil
                host?.notify(problem)
            } else {
                asking = true
            }
        }
    }

    private var repository: String? { askedFolder ?? folder }

    private func replan() {
        guard let folder = repository else { return }
        planning?.cancel()
        error = nil
        guard !branch.isEmpty else { return plan = nil }
        planning = Task {
            let next = await backend.plan(branch: branch, repo: folder)
            if !Task.isCancelled { plan = next }
        }
    }

    public func create() {
        guard let folder = repository, canCreate else { return }
        busy = true
        Task {
            let failure = await backend.create(branch: branch, repo: folder)
            busy = false
            if let failure { error = failure } else { asking = false }
        }
    }

    /// Remove Worktree: asks first, and not at all while the folder has changes.
    public func askRemoval() { if made != nil, !dirty { confirmingRemoval = true } }

    public var removalNote: String? {
        guard let made else { return nil }
        return "Its worktree \(made.branch) at \(made.folder) was made by Wormhole. Remove deletes its folder (the branch stays)."
    }

    public func confirmRemoval() {
        guard let worktree = made else { return }
        Task {
            if let failure = await backend.remove(worktree) { host?.notify("Couldn't remove the worktree: \(failure)") } else { made = nil }
        }
    }
}
