// What the right-click menu of a commit offers (`features/git/history/menu.rs`): the copies of its hash, message and
// author, and a way to its parent.

import PanelModel
import SwiftUI

extension View {
    func commitMenu(_ commit: Commit, store: HistoryStore, openParent: @escaping (String) -> Void) -> some View {
        modifier(CommitMenu(commit: commit, store: store, openParent: openParent))
    }
}

/// The copies a commit offers, as a menu's items: shared by the row's menu and the details' "More ways to copy".
struct CommitCopies: View {
    let commit: Commit
    let store: HistoryStore
    @Environment(\.panelCopy) private var copy

    var body: some View {
        item(.hash, panelText("history-menu-copy-hash", "Copy Hash"))
        item(.shortHash, panelText("history-menu-copy-short-hash", "Copy Short Hash"))
        item(.message, panelText("history-menu-copy-message", "Copy Message"))
        item(.subject, panelText("history-menu-copy-subject", "Copy Subject"))
        item(.author, panelText("history-menu-copy-author", "Copy Author"))
    }

    private func item(_ what: CommitCopy, _ title: String) -> some View {
        Button(title) { Task { if let text = await store.text(of: what, for: commit) { copy(text) } } }
    }
}

private struct CommitMenu: ViewModifier {
    let commit: Commit
    let store: HistoryStore
    let openParent: (String) -> Void

    func body(content: Content) -> some View {
        content.contextMenu {
            CommitCopies(commit: commit, store: store)
            if let parent = commit.parents.first {
                Divider()
                Button(panelText("history-menu-open-parent", "Open Parent \(parent.prefix(7))")) { openParent(parent) }
            }
        }
    }
}
