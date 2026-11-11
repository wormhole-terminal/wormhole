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
    @Environment(\.panelHost) private var host

    /// The five copies in the menu's order, with their titles.
    static var titles: [(CommitCopy, String)] {
        [
            (.hash, panelText("history-menu-copy-hash", "Copy Hash")),
            (.shortHash, panelText("history-menu-copy-short-hash", "Copy Short Hash")),
            (.message, panelText("history-menu-copy-message", "Copy Message")),
            (.subject, panelText("history-menu-copy-subject", "Copy Subject")),
            (.author, panelText("history-menu-copy-author", "Copy Author")),
        ]
    }

    var body: some View {
        ForEach(Self.titles, id: \.1) { what, title in item(what, title) }
    }

    private func item(_ what: CommitCopy, _ title: String) -> some View {
        Button(title) { Task { if let text = await store.text(of: what, for: commit) { copy(text, store.copied(what, for: commit)) } else { host?.failed(.copyCommit) } } }
    }
}

private struct CommitMenu: ViewModifier {
    let commit: Commit
    let store: HistoryStore

    func body(content: Content) -> some View {
        content.contextMenu { CommitCopies(commit: commit, store: store) }
    }
}
