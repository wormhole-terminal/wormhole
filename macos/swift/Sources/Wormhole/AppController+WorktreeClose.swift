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
        let outside = session.hub.state.sections.flatMap(\.rows).map(\.terminal).filter { other in !terminals.contains { $0.id == other.id } }
        var found: [(made: MadeWorktree, removable: Bool)] = []
        for terminal in terminals {
            guard let folder = Self.expanded(terminal.folder), let made = await session.hub.panel.worktrees.made(at: folder),
                !found.contains(where: { $0.made.folder == made.folder })
            else { continue }
            let shared = outside.contains { Self.expanded($0.folder).map { $0.hasPrefix(made.folder) } ?? false }
            found.append((made, !terminal.isDirty && (!group || !shared)))
        }
        guard let first = found.first else { return nil }
        let home = NSHomeDirectory()
        let shown = { (path: String) in path.hasPrefix(home) ? "~" + path.dropFirst(home.count) : path }
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

    /// The sidebar writes a folder with `~`; the panel and git read real paths.
    static func expanded(_ folder: String?) -> String? {
        guard let folder else { return nil }
        return folder == "~" ? NSHomeDirectory() : folder.hasPrefix("~/") ? NSHomeDirectory() + folder.dropFirst(1) : folder
    }
}
