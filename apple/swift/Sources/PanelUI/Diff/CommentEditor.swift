// The small editor a click on a diff line opens (`review.rs`'s dialog): the line quoted, a field for what the agent
// should change there, Remove for an old comment, Cancel and Add Comment (or Save). ⌘↩ saves, esc cancels.

import DesignSystem
import PanelModel
import SwiftUI

struct CommentEditor: View {
    let place: String
    let at: LineAt
    let existing: ReviewComment?
    let save: (String) -> Void
    let remove: () -> Void
    let cancel: () -> Void
    @State private var text: String
    @FocusState private var focused: Bool
    @Environment(\.tokens) private var tokens
    @Environment(\.codeFont) private var codeFont

    init(place: String, at: LineAt, existing: ReviewComment?, save: @escaping (String) -> Void, remove: @escaping () -> Void, cancel: @escaping () -> Void) {
        (self.place, self.at, self.existing, self.save, self.remove, self.cancel) = (place, at, existing, save, remove, cancel)
        _text = State(initialValue: existing?.text ?? "")
    }

    var body: some View {
        VStack(alignment: .leading, spacing: PanelMetrics.gap) {
            Text(panelText("review-title", ["place": .text(place)], "Comment on \(place)")).font(TextRole.label.font).foregroundStyle(tokens.textPrimary)
            let code = at.code.trimmingCharacters(in: .whitespaces)
            if !code.isEmpty {
                Text(code).font(codeFont.font(TextRole.caption.size)).foregroundStyle(tokens.textMuted).lineLimit(1).truncationMode(.tail)
                    .padding(.horizontal, Space._2).padding(.vertical, Space._1).frame(maxWidth: .infinity, alignment: .leading)
                    .background(tokens.stateActive, in: RoundedRectangle(cornerRadius: tokens.metrics.radiusSm))
            }
            TextEditor(text: $text).font(TextRole.body.font).scrollContentBackground(.hidden).focused($focused)
                .frame(height: PanelMetrics.noteLine * 4).padding(Space._1)
                .background(tokens.inputBackground, in: RoundedRectangle(cornerRadius: tokens.metrics.radius))
                .overlay { RoundedRectangle(cornerRadius: tokens.metrics.radius).strokeBorder(focused ? tokens.inputBorderFocus : tokens.inputBorder, lineWidth: tokens.metrics.borderWidth) }
                .overlay(alignment: .topLeading) {
                    if text.isEmpty {
                        Text(panelText("review-placeholder", "What should the agent change here?")).font(TextRole.body.font)
                            .foregroundStyle(tokens.inputPlaceholder).padding(Space._2).allowsHitTesting(false)
                    }
                }
            HStack(spacing: PanelMetrics.gap) {
                KeyCap(text: "\u{2318}\u{21a9}", height: Metric.titleControl)
                Text(panelText("review-saves", "saves")).font(TextRole.caption.font).foregroundStyle(tokens.textMuted)
                Spacer(minLength: 0)
                if existing != nil { NoticeButton(title: panelText("review-remove", "Remove"), action: remove) }
                NoticeButton(title: panelText("dialog-cancel", "Cancel"), action: cancel)
                NoticeButton(title: existing == nil ? panelText("review-add", "Add Comment") : panelText("review-save", "Save"), primary: true) { save(text) }
                    .keyboardShortcut(.return, modifiers: .command)
            }
        }
        .padding(Space._3).frame(width: 340)
        .background(tokens.surfaceOverlay)
        .onAppear { focused = true }
        .onExitCommand(perform: cancel)
    }
}
