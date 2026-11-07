// A worktree per terminal, as the panel hosts it: the sheet that asks for a branch before making one, and the
// confirmation before removing the one Wormhole made for this terminal. Writes only on the person's action, and
// the backend refuses in a project that isn't trusted.

import Observation
import PanelModel

@MainActor @Observable
public final class WorktreeStore {
    public private(set) var folder: String?
    /// The new-worktree sheet is up.
    public var asking = false
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

    public func ask() {
        guard let folder else { return }
        branch = ""
        plan = nil
        error = nil
        Task {
            if let problem = await backend.problem(making: folder) { host?.notify(problem) } else { asking = true }
        }
    }

    private func replan() {
        guard let folder else { return }
        planning?.cancel()
        error = nil
        guard !branch.isEmpty else { return plan = nil }
        planning = Task {
            let next = await backend.plan(branch: branch, repo: folder)
            if !Task.isCancelled { plan = next }
        }
    }

    public func create() {
        guard let folder, canCreate else { return }
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
