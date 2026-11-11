// The right panel's git over the Rust core: `GitBackend` and `WorktreeBackend` made from one call, for the window
// to hand to `PanelBackends`. Linked builds only (the module exists when the FFI is linked, as UniFFISource does).

import PanelModel
import WormholeBindings

public struct GitBackends: Sendable {
    public let git: RealGitBackend
    public let worktrees: RealWorktreeBackend
}

public enum GitBridge {
    /// Throws when the process is not an isolated instance (`WORMHOLE_DATA_DIR` named `Wormhole-swift*`).
    /// - Parameters:
    ///   - worktreeAllowed: why a worktree may not be made of a folder (the project is not trusted), or nil.
    ///     Required: git does not know about project trust, so nothing is written unless the window says so.
    ///   - worktreeCreated: where the window opens the terminal for a worktree that was just made.
    ///   - worktreeBase: the setting `worktree_base` (`~` already expanded); nil puts them beside the repository.
    ///   - binarySizeUnits: the setting `size_units`: KiB rather than KB in a diff's words.
    public static func makeBackends(
        worktreeAllowed: @escaping RealWorktreeBackend.Allowed,
        worktreeCreated: @escaping RealWorktreeBackend.Created = { _, _ in },
        worktreeBase: String? = nil, cloneBuildFolders: Bool = false, binarySizeUnits: Bool = false
    ) throws -> GitBackends {
        let git = try Git()
        git.setBinaryUnits(binary: binarySizeUnits)
        let service = try WorktreeService(base: worktreeBase, cloneBuildFolders: cloneBuildFolders)
        return GitBackends(
            git: RealGitBackend(git),
            worktrees: RealWorktreeBackend(service, allowed: worktreeAllowed, created: worktreeCreated))
    }
}
