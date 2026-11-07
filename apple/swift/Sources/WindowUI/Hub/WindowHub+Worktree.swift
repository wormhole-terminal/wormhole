// The question `wh new-terminal --worktree <branch>` puts up (the GPUI app's `control_ask`): the panel's new-worktree
// sheet with the branch filled in, for the repository the command was run in. The sheet belongs to the panel, so the
// panel is shown first; nothing is made until the person presses Create.

import WorkspaceModel

extension WindowHub {
    public func askWorktree(branch: String, in folder: String) {
        send(.showPanelTab(.changes))
        panel.worktrees.ask(branch: branch, in: folder)
    }
}
