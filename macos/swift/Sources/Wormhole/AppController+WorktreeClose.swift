// The worktree lines of the close question (`features/git/worktree/close.rs`): a terminal in a worktree Wormhole made is asked
// about even when nothing runs in it. Close keeps the worktree; Close and Remove Worktree deletes its folder too (the branch
// stays), not offered while the folder has changes as last read (git refuses again by itself if it has some by then). For a group,
// each worktree once, removable only without changes and without a terminal outside the group in it.

import AppKit
import AuxModel
import PanelModel
import WorkspaceModel

struct WorktreeClose {
    var note: String
    /// The extra button's label, nil when nothing may be removed.
    var also: String?
    var remove: [MadeWorktree]
}

extension AppController {
    /// What closing `terminals` adds to the question, or nil when none of them is in a worktree Wormhole made.
    func worktreeClose(of terminals: [TerminalSummary], in session: WindowSession, group: Bool) async -> WorktreeClose? {
        // Every terminal of the window, the ones in a folded group too (`state.all`, not the rows the list shows).
        let outside = session.hub.state.all.filter { other in !terminals.contains { $0.id == other.id } }
        var found: [(made: MadeWorktree, removable: Bool)] = []
        for terminal in terminals {
            // The core's path (a key where a name is not text), as the worktree was recorded: never the drawn folder,
            // whose U+FFFD names no folder there, or another.
            guard let folder = terminal.readPath, let made = await session.hub.panel.worktrees.made(at: folder),
                !found.contains(where: { $0.made.folder == made.folder })
            else { continue }
            let shared = outside.contains { $0.readPath.map { Self.inside($0, made.folder) } ?? false }
            found.append((made, !terminal.isDirty && (!group || !shared)))
        }
        guard let first = found.first else { return nil }
        let home = HomeFolder.path
        // A folder whose name is not text comes as a key (`DiskName`): drawn with U+FFFD for its bytes.
        let shown = { (key: String) in
            let path = DiskName.shown(key)
            return path.hasPrefix(home) ? "~" + path.dropFirst(home.count) : path
        }
        guard group else {
            let dirty = !first.removable
            let folder = shown(first.made.folder)
            let note = dirty ? AuxText.worktreeDirty(branch: first.made.branch, folder: folder) : AuxText.worktreeClean(branch: first.made.branch, folder: folder)
            return WorktreeClose(note: note, also: dirty ? nil : AuxText.worktreeRemoveOne, remove: dirty ? [] : [first.made])
        }
        let names = found.map { AuxText.worktreeEntry(branch: $0.made.branch, folder: shown($0.made.folder)) }.joined(separator: ", ")
        let removable = found.filter(\.removable).map(\.made)
        var note = AuxText.worktreeGroupLead(count: found.count, names: names) + " "
        switch (removable.count, found.count - removable.count) {
        case (0, _): note += AuxText.worktreeGroupNone
        case (_, 0): note += AuxText.worktreeGroupAll
        default: note += AuxText.worktreeGroupSome
        }
        return WorktreeClose(note: note, also: removable.isEmpty ? nil : AuxText.worktreeRemoveMany, remove: removable)
    }

    /// Whether `path` is the folder or under it, by whole path components: `…/fix-login2` is not inside `…/fix-login`.
    static func inside(_ path: String, _ folder: String) -> Bool {
        path == folder || path.hasPrefix(folder.hasSuffix("/") ? folder : folder + "/")
    }
}
