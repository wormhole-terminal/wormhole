// The backends the panel reads through, gathered so the window hands them over once. Each is a seam the FFI fills
// with the Rust core; `mock` is the sample data of previews, screenshots and tests.

import PanelModel

public struct PanelBackends: Sendable {
    public var git: any GitBackend
    public var files: any FilesBackend
    public var project: any ProjectBackend
    public var usage: any UsageBackend
    public var worktrees: any WorktreeBackend
    public var syntax: any SyntaxHighlighting

    public init(
        git: any GitBackend, files: any FilesBackend, project: any ProjectBackend, usage: any UsageBackend,
        worktrees: any WorktreeBackend, syntax: any SyntaxHighlighting = LexerHighlighter()
    ) {
        (self.git, self.files, self.project, self.usage, self.worktrees, self.syntax) = (git, files, project, usage, worktrees, syntax)
    }

    public static func mock(made: MadeWorktree? = nil) -> PanelBackends {
        PanelBackends(
            git: MockGitBackend(), files: MockFilesBackend(pinned: ["/Users/wanderer/Projects/wormhole/docs"]),
            project: MockProjectBackend(), usage: MockUsageBackend(), worktrees: MockWorktreeBackend(made: made))
    }
}
