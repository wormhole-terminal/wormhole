// A next step (`features/notes/task_rows.rs`, `style::item_row`): the box in the lead column and its words in the
// body style, muted and struck through once done. Clicking the words edits them in place; ↵ keeps the change,
// esc drops it, and emptying the field removes the step. The actions show under the pointer.

import DesignSystem
import PanelModel
import SwiftUI

struct TaskRowView: View {
    let task: PanelTask
    let toggle: () -> Void
    let rename: (String) -> Void
    let send: () -> Void
    let delete: () -> Void
    var moveBy: (Int) -> Void = { _ in }
    @State private var hovering = false
    @State private var editing = false
    @State private var draft = ""
    @FocusState private var focused: Bool
    @Environment(\.tokens) private var tokens
    @Environment(\.panelMotion) private var motion

    var body: some View {
        HStack(spacing: PanelMetrics.gap) {
            Button(action: toggle) { TaskCheckbox(checked: task.done).contentShape(Rectangle()) }
                .buttonStyle(.plain)
                .accessibilityLabel(task.text)
                .accessibilityValue(task.done ? "done" : "not done")
            label
            if hovering || editing { RowActions(send: send, delete: delete).transition(.opacity) }
        }
        .padding(.horizontal, PanelMetrics.inset)
        .frame(minHeight: Metric.row)
        .background(hovering || editing ? tokens.stateHover : .clear, in: RoundedRectangle(cornerRadius: Radius.row, style: .continuous))
        .onHover { hovering = $0 }
        .animation(motion.tick, value: task.done)
    }

    @ViewBuilder private var label: some View {
        if editing {
            TextField("", text: $draft)
                .textFieldStyle(.plain).font(TextRole.body.font).focused($focused)
                .onSubmit { finish(keep: true) }
                .onExitCommand { finish(keep: false) }
                .reorderKeys(moveBy)
                .onChange(of: focused) { _, now in if !now && editing { finish(keep: true) } }
        } else {
            Text(task.text).font(TextRole.body.font)
                .foregroundStyle(task.done ? tokens.textFaint : tokens.textPrimary)
                .strikethrough(task.done, color: tokens.textFaint)
                .lineLimit(1).truncationMode(.tail)
                .frame(maxWidth: .infinity, alignment: .leading)
                .contentShape(Rectangle())
                .onTapGesture { draft = task.text; editing = true; focused = true }
        }
    }

    private func finish(keep: Bool) {
        editing = false
        if keep, draft != task.text { rename(draft) }
    }
}
