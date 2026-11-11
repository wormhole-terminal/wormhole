// What a worktree asks and says in the panel (`features/git/worktree/ask.rs`): the sheet that asks for the branch before making
// one in a folder of its own, and the alerts that say why none was made. Nothing is written until the person confirms.

import DesignSystem
import PanelModel
import SwiftUI

extension View {
    /// The worktree's sheet and alerts, on the window rather than the panel: a worktree closed away with the panel hidden still
    /// says why it was not removed.
    public func worktreeSheets(_ store: WorktreeStore) -> some View {
        modifier(WorktreeSheets(store: store))
    }
}

private struct WorktreeSheets: ViewModifier {
    @Bindable var store: WorktreeStore

    func body(content: Content) -> some View {
        content
            .sheet(isPresented: $store.asking) { NewWorktreeSheet(store: store) }
            .alert(item: $store.alert) { alert in Alert(title: Text(alert.title), message: Text(alert.message)) }
    }
}

/// "New terminal in a new worktree": a branch field, the folder it would make and the reason a branch is refused.
struct NewWorktreeSheet: View {
    @Bindable var store: WorktreeStore
    @FocusState private var focused: Bool
    @Environment(\.tokens) private var tokens

    var body: some View {
        VStack(alignment: .leading, spacing: Space._3) {
            Text(panelText("worktree-title", "New terminal in a new worktree")).font(TextRole.heading.font).foregroundStyle(tokens.textPrimary)
            Text(panelText("worktree-about", "A worktree of \(store.repositoryName) on a branch of its own, in a folder of its own, so work there touches nothing in the repository's folder."))
                .font(TextRole.body.font).foregroundStyle(tokens.textMuted).fixedSize(horizontal: false, vertical: true)
            VStack(alignment: .leading, spacing: Space._1) {
                Text(panelText("worktree-branch", "Branch (new, or one the repository has)")).font(TextRole.caption.font.weight(.medium)).foregroundStyle(tokens.textMuted)
                TextField("fix/login", text: $store.branch).textFieldStyle(.roundedBorder).focused($focused).onSubmit(store.create)
                if let note = store.footnote {
                    Text(note).font(TextRole.caption.font).foregroundStyle(isRefusal ? tokens.statusWarningText : tokens.textMuted)
                        .fixedSize(horizontal: false, vertical: true)
                }
            }
            HStack(spacing: PanelMetrics.gap) {
                KeyCap(text: "esc", height: Metric.titleControl)
                Text(panelText("worktree-cancels", "cancels")).font(TextRole.caption.font).foregroundStyle(tokens.textMuted)
                Spacer(minLength: 0)
                NoticeButton(title: panelText("worktree-cancel", "Cancel")) { store.asking = false }.keyboardShortcut(.cancelAction)
                NoticeButton(title: panelText("worktree-create", "Create"), primary: true, busy: !store.canCreate, action: store.create)
                    .keyboardShortcut(.defaultAction)
            }
        }
        .padding(Space._4).frame(width: 420).background(tokens.surfaceOverlay)
        .onAppear { focused = true }
    }

    private var isRefusal: Bool { if case .refused = store.plan { true } else { false } }
}
