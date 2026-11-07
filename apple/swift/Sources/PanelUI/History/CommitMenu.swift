// What the right-click menu of a commit offers (`features/git/history/menu.rs`): the copies of its hash, message and
// author, and a way to its parent.

import PanelModel
import SwiftUI

extension View {
    func commitMenu(_ commit: Commit, openParent: @escaping (String) -> Void) -> some View {
        modifier(CommitMenu(commit: commit, openParent: openParent))
    }
}

/// The copies a commit offers, as a menu's items: shared by the row's menu and the details' "More ways to copy".
struct CommitCopies: View {
    let commit: Commit
    @Environment(\.panelCopy) private var copy

    var body: some View {
        Button(panelText("history-menu-copy-hash", "Copy Hash")) { copy(commit.sha) }
        Button(panelText("history-menu-copy-short-hash", "Copy Short Hash")) { copy(commit.short) }
        Button(panelText("history-menu-copy-message", "Copy Message")) { copy(commit.body.isEmpty ? commit.subject : commit.subject + "\n\n" + commit.body) }
        Button(panelText("history-menu-copy-subject", "Copy Subject")) { copy(commit.subject) }
        Button(panelText("history-menu-copy-author", "Copy Author")) { copy("\(commit.author.name) <\(commit.author.email)>") }
    }
}

private struct CommitMenu: ViewModifier {
    let commit: Commit
    let openParent: (String) -> Void

    func body(content: Content) -> some View {
        content.contextMenu {
            CommitCopies(commit: commit)
            if let parent = commit.parents.first {
                Divider()
                Button(panelText("history-menu-open-parent", "Open Parent \(parent.prefix(7))")) { openParent(parent) }
            }
        }
    }
}
