// A git worktree per terminal (`wormhole_core::git::worktree`): the one place Wormhole writes to a repository, kept
// narrow and run only on the person's action, confirmed in a sheet, never in an untrusted project.

public struct MadeWorktree: Hashable, Sendable {
    public var repo: String
    public var branch: String
    public var folder: String

    public init(repo: String, branch: String, folder: String) {
        (self.repo, self.branch, self.folder) = (repo, branch, folder)
    }
}

/// What a typed branch would make.
public enum WorktreePlan: Equatable, Sendable {
    /// The branch name git accepts, and the folder the worktree would go in.
    case ready(branch: String, folder: String)
    case refused(String)
}

public protocol WorktreeBackend: Sendable {
    /// Why a worktree can't be made of `folder` (not a repository, not trusted), or nil.
    func problem(making folder: String) async -> String?
    /// The branch name as git's rules read it, and where the folder goes (beside the repository or under the
    /// configured base), without writing anything.
    func plan(branch typed: String, repo folder: String) async -> WorktreePlan
    /// Makes it and opens a terminal in it. The error is git's words.
    /// `command` (`--command`) runs in that terminal after the project's setup.
    func create(branch typed: String, repo folder: String, command: String?) async -> String?
    /// The worktree Wormhole made that `folder` is in, if any.
    func made(at folder: String) async -> MadeWorktree?
    /// Removes the folder (the branch stays); refused while it has changes. The error is git's words.
    func remove(_ worktree: MadeWorktree) async -> String?
    /// The settings `worktree_base` (as written, `~` and all) and `worktree_clone_build_folders`.
    func setOptions(base: String?, cloneBuildFolders: Bool)
    /// The window is in profile `id`: the worktrees it makes and remembers (and their ports) are that profile's.
    func followProfile(_ id: String)
}

extension WorktreeBackend {
    public func setOptions(base: String?, cloneBuildFolders: Bool) {}
    public func followProfile(_ id: String) {}
}
