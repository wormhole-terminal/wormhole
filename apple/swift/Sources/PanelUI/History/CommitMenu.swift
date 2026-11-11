// What the right-click menu of a commit row offers (`features/git/history/menu.rs`): the five copies, and nothing else (the
// parents are the details block's).

import PanelModel
import SwiftUI

extension View {
    func commitMenu(_ commit: Commit, store: HistoryStore) -> some View {
        modifier(CommitMenu(commit: commit, store: store))
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

    func body(content: Content) -> some View {
        content.contextMenu { CommitCopies(commit: commit, store: store) }
    }
}
