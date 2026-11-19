// The editor a click on a diff line opens, a sheet on the window as `review.rs`'s dialog is: the line quoted, a field for what
// the agent should change there, Remove for an old comment, Cancel and Add Comment (or Save). ⌘↩ saves, esc cancels. What
// it shows and edits is `ChangesStore.commenting`, never a row's: the rows are a lazy list a reload of the diff rebuilds.

import DesignSystem
import PanelModel
import SwiftUI

extension View {
    /// The comment being written on a diff line, as a sheet on the window rather than on its row: a reload of the diff
    /// (an agent editing that file), the diff closing or the panel hiding leaves it up with what was typed.
    public func commentSheet(_ store: ChangesStore) -> some View {
        modifier(CommentSheet(store: store))
    }
}

private struct CommentSheet: ViewModifier {
    let store: ChangesStore

    func body(content: Content) -> some View {
        let open = store.commenting != nil
        content.sheet(isPresented: Binding(get: { open }, set: { if !$0 { store.commenting = nil } })) {
            if let comment = store.commenting { CommentEditor(store: store, comment: comment) }
        }
    }
}

struct CommentEditor: View {
    let store: ChangesStore
    /// What is being written, as the store holds it now.
    let comment: ReviewComment
    @FocusState private var focused: Bool
    @Environment(\.tokens) private var tokens
    @Environment(\.isEnabled) private var enabled
    @Environment(\.codeFont) private var codeFont

    private var place: String { comment.location }
    private var existing: Bool { store.commentingExisting }
    private var text: Binding<String> { Binding(get: { store.commenting?.text ?? "" }, set: { store.commenting?.text = $0 }) }

    var body: some View {
        VStack(alignment: .leading, spacing: PanelMetrics.gap) {
            Text(panelText("review-title", ["place": .text(place)], "Comment on \(place)")).font(tokens.font(.label)).foregroundStyle(tokens.textPrimary)
            let code = comment.code.trimmingCharacters(in: .whitespaces)
            if !code.isEmpty {
                Text(code).font(codeFont.font(tokens.size(.caption))).foregroundStyle(tokens.textMuted).lineLimit(1).truncationMode(.tail)
                    .padding(.horizontal, Space._2).padding(.vertical, Space._1).frame(maxWidth: .infinity, alignment: .leading)
                    .background(tokens.stateActive, in: RoundedRectangle(cornerRadius: tokens.metrics.radiusSm))
            }
            TextEditor(text: text).font(tokens.font(.body)).scrollContentBackground(.hidden).focused($focused)
                .frame(height: PanelMetrics.noteLine * 4).padding(Space._1)
                .background(tokens.inputBackground, in: RoundedRectangle(cornerRadius: tokens.metrics.radius))
                .overlay { RoundedRectangle(cornerRadius: tokens.metrics.radius).strokeBorder(tokens.inputBorder, lineWidth: tokens.metrics.borderWidth) }
                .flatFocusRing(focused && enabled, radius: tokens.metrics.radius)
                .opacity(enabled ? 1 : 0.5)
                .overlay(alignment: .topLeading) {
                    if comment.text.isEmpty {
                        Text(panelText("review-placeholder", "What should the agent change here?")).font(tokens.font(.body))
                            .foregroundStyle(tokens.inputPlaceholder).padding(Space._2).allowsHitTesting(false)
                    }
                }
            HStack(spacing: PanelMetrics.gap) {
                KeyCap(text: "\u{2318}\u{21a9}", height: Metric.titleControl)
                Text(panelText("review-saves", "saves")).font(tokens.font(.caption)).foregroundStyle(tokens.textMuted)
                Spacer(minLength: 0)
                if existing { NoticeButton(title: panelText("review-remove", "Remove"), action: { store.removeComment() }) }
                NoticeButton(title: panelText("dialog-cancel", "Cancel")) { store.commenting = nil }
                NoticeButton(title: existing ? panelText("review-save", "Save") : panelText("review-add", "Add Comment"), primary: true) { store.saveComment() }
                    .keyboardShortcut(.return, modifiers: .command)
            }
        }
        .padding(Space._3).frame(width: 340)
        .background(tokens.surfaceOverlay)
        .onAppear { focused = true }
        .onExitCommand { store.commenting = nil }
    }
}
