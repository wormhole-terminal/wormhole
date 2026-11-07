import Foundation

// A `WorktreeBackend` that makes nothing: it plans the way core does (the folder beside the repository) and
// remembers what it was asked, for previews and tests.

public actor MockWorktreeBackend: WorktreeBackend {
    public private(set) var created: [String] = []
    public private(set) var removed: [MadeWorktree] = []
    var madeHere: MadeWorktree?
    var refusal: String?

    public init(made: MadeWorktree? = nil, refusal: String? = nil) {
        madeHere = made
        self.refusal = refusal
    }

    public func problem(making folder: String) async -> String? { nil }

    public func plan(branch typed: String, repo folder: String) async -> WorktreePlan {
        let branch = typed.trimmingCharacters(in: .whitespaces)
        guard !branch.isEmpty, !branch.contains(" "), !branch.hasPrefix("-") else { return .refused("That isn't a branch name git accepts.") }
        let name = folder.split(separator: "/").last.map(String.init) ?? "repo"
        let parent = folder.split(separator: "/").dropLast().joined(separator: "/")
        return .ready(branch: branch, folder: "/\(parent)/\(name)-worktrees/\(branch.replacingOccurrences(of: "/", with: "-"))")
    }

    public func create(branch typed: String, repo folder: String) async -> String? {
        if let refusal { return refusal }
        created.append(typed)
        return nil
    }

    public func made(at folder: String) async -> MadeWorktree? { madeHere }

    public func remove(_ worktree: MadeWorktree) async -> String? {
        removed.append(worktree)
        madeHere = nil
        return nil
    }
}
