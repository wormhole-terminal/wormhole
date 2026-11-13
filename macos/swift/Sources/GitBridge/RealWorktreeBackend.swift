// `WorktreeBackend` over core's `git::worktree`: the one place Wormhole writes to a repository, `worktree add`
// and `worktree remove` through `GitWrite` (hooks off, never forced), only for the calls the person confirmed.
// Whether the *project* trusts a worktree lives in the project feature, not in git: the window passes it in as
// `allowed`, and it is asked again right before the write, as the dialog does.

import PanelModel
import WormholeBindings

public struct RealWorktreeBackend: WorktreeBackend {
    /// A reason a worktree may not be made of this folder (an untrusted project), or nil.
    public typealias Allowed = @Sendable (_ folder: String) async -> String?
    /// A worktree was made: the window opens a terminal in its folder, with these variables (its own ports), and runs
    /// `command` (`--command`) in it after the project's setup.
    public typealias Created = @Sendable (_ made: MadeWorktree, _ environment: [String: String], _ command: String?) async -> Void

    private let service: WorktreeService
    private let allowed: Allowed
    private let created: Created

    init(_ service: WorktreeService, allowed: @escaping Allowed, created: @escaping Created) {
        (self.service, self.allowed, self.created) = (service, allowed, created)
    }

    /// The settings `worktree_base` and `worktree_clone_build_folders` changed.
    public func setOptions(base: String?, cloneBuildFolders: Bool) {
        service.setOptions(base: base, cloneBuildFolders: cloneBuildFolders)
    }

    /// The window's profile made the worktrees of this service: they are remembered in its database.
    public func followProfile(_ id: String) { _ = service.followProfile(profile: id) }

    /// Trust is the repository's (its main working tree, which a terminal in a linked worktree is of too), never the
    /// terminal's folder: `.wormhole/` is looked for there.
    private func repository(of folder: String) async -> String {
        await GitQueue.run { [service] in service.repository(folder: folder) } ?? folder
    }

    public func problem(making folder: String) async -> String? {
        if let why = await allowed(await repository(of: folder)) { return why }
        return await GitQueue.run { [service] in service.problem(folder: folder) }
    }

    public func plan(branch typed: String, repo folder: String) async -> WorktreePlan {
        switch await GitQueue.run({ [service] in service.plan(branch: typed, folder: folder) }) {
        case let .ready(branch, folder): .ready(branch: branch, folder: folder)
        case let .refused(reason): .refused(reason)
        }
    }

    public func create(branch typed: String, repo folder: String, command: String?) async -> String? {
        if let why = await allowed(await repository(of: folder)) { return why }
        do {
            let made = try await GitQueue.run { [service] in try service.create(branch: typed, folder: folder) }
            let environment = Dictionary(made.env.map { ($0.name, $0.value) }, uniquingKeysWith: { $1 })
            await created(MadeWorktree(made), environment, command)
            return nil
        } catch {
            return (error as? GitFailure)?.message ?? "\(error)"
        }
    }

    public func made(at folder: String) async -> MadeWorktree? {
        await GitQueue.run { [service] in service.madeAt(folder: folder) }.map(MadeWorktree.init)
    }

    public func remove(_ worktree: MadeWorktree) async -> String? {
        do {
            try await GitQueue.run { [service] in try service.remove(folder: worktree.folder) }
            return nil
        } catch {
            return (error as? GitFailure)?.message ?? "\(error)"
        }
    }
}

extension MadeWorktree {
    init(_ made: GitWorktree) { self.init(repo: made.repo, branch: made.branch, folder: made.folder) }
}
