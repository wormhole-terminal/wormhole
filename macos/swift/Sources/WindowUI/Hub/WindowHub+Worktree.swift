// The question `wh new-terminal --worktree <branch>` puts up (the GPUI app's `control_ask`): the panel's new-worktree
// sheet with the branch filled in, for the repository the command was run in. Nothing is made until the person presses Create.

import WorkspaceModel

extension WindowHub {
    public func askWorktree(branch: String, in folder: String, command: String? = nil) {
        panel.worktrees.ask(branch: branch, in: folder, command: command)
    }
}
