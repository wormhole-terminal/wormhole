// The close question's buttons and the worktree lines it adds (`swift-aux.ftl`).

import Foundation
import WordsModel

extension AuxText {
    public static var closeConfirmSession: String { tr("close-confirm-session", "Close") }
    public static var closeConfirmGroup: String { tr("close-confirm-group", "Close Group") }
    public static var closeWindowTitle: String { tr("close-window-title", "Close this window?") }
    public static var closeConfirmWindow: String { tr("close-confirm-window", "Close Window") }
    /// The open panel's button.
    public static var openFolderPrompt: String { tr("open-folder-prompt", "Open") }

    public static func worktreeDirty(branch: String, folder: String) -> String {
        tr("worktree-close-one-dirty", ["branch": .text(branch), "folder": .text(folder)], fallback: "Its worktree \(branch) at \(folder) was made by Wormhole. It has uncommitted changes, so it stays.")
    }
    public static func worktreeClean(branch: String, folder: String) -> String {
        tr("worktree-close-one-clean", ["branch": .text(branch), "folder": .text(folder)], fallback: "Its worktree \(branch) at \(folder) was made by Wormhole. Close keeps it; Close and Remove Worktree deletes its folder (the branch stays).")
    }
    public static var worktreeRemoveOne: String { tr("worktree-close-remove-one", "Close and Remove Worktree") }
    public static var worktreeRemoveMany: String { tr("worktree-close-remove-many", "Close and Remove Worktrees") }
    public static func worktreeEntry(branch: String, folder: String) -> String {
        tr("worktree-close-entry", ["branch": .text(branch), "folder": .text(folder)], fallback: "\(branch) at \(folder)")
    }
    public static func worktreeGroupLead(count: Int, names: String) -> String {
        count == 1
            ? tr("worktree-close-group-lead-one", ["names": .text(names)], fallback: "Wormhole made the worktree \(names).")
            : tr("worktree-close-group-lead-many", ["names": .text(names)], fallback: "Wormhole made the worktrees \(names).")
    }
    public static var worktreeGroupNone: String { tr("worktree-close-group-none", "They have changes or other terminals in them, so they stay.") }
    public static var worktreeGroupAll: String { tr("worktree-close-group-all", "Close keeps them; Close and Remove Worktrees deletes their folders (the branches stay).") }
    public static var worktreeGroupSome: String { tr("worktree-close-group-some", "Close keeps them; Close and Remove Worktrees deletes the folders without changes or other terminals (the branches stay).") }
}
