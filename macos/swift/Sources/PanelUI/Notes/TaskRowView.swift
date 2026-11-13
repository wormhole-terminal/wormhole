// A next step (`features/notes/task_rows.rs`, `style::item_row`): the box in the lead column and its words in the
// body style, muted and struck through once done. Clicking the words edits them in place; ↵ keeps the change and opens
// the next step below (on an empty field it just ends the writing), esc keeps it and gives the keys back to the terminal,
// ⌘↵ sends the words to the prompt (⇧ also submits), ⌫ on an empty field deletes the step and opens the one above, and
// emptying the field removes it. The actions show under the pointer.

import DesignSystem
import PanelModel
import SwiftUI

struct TaskRowView: View {
    let task: PanelTask
    let store: NotesStore
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
                .accessibilityValue(task.done ? panelText("notes-task-done", "done") : panelText("notes-task-not-done", "not done"))
            label
            // While its editor is open the row's Send sends what is typed, not the words saved before (`task_rows.rs`).
            if hovering || editing {
                RowActions(send: { if editing { store.send(text: draft, submit: false) } else { send() } }, delete: delete).transition(.opacity)
            }
        }
        .padding(.horizontal, PanelMetrics.inset)
        .frame(minHeight: Metric.row)
        .background(hovering || editing ? tokens.stateHover : .clear, in: RoundedRectangle(cornerRadius: Radius.row, style: .continuous))
        .marked("panel.item")
        .onHover { hovering = $0 }
        .animation(motion.tick, value: task.done)
        .onChange(of: editing) { _, now in store.setWriting(task.id, now) }
        .onDisappear { store.setWriting(task.id, false) }
        .onChange(of: store.freshStep) { _, id in
            if id == task.id { edit(); store.consumeFreshStep() }
        }
    }

    @ViewBuilder private var label: some View {
        if editing {
            TextField("", text: $draft)
                .textFieldStyle(.plain).font(TextRole.body.font).focused($focused)
                .onSubmit {
                    let empty = draft.trimmingCharacters(in: .whitespaces).isEmpty
                    finish(keep: true)
                    if empty { store.leave() } else { (store.draftAfter, store.drafting) = (task.id, true) }
                }
                .onExitCommand { finish(keep: true); store.leave() }
                .onKeyPress(.return, phases: .down) { press in
                    guard press.modifiers.contains(.command) else { return .ignored }
                    store.send(text: draft, submit: press.modifiers.contains(.shift))
                    return .handled
                }
                .onKeyPress(.delete, phases: .down) { _ in
                    guard draft.isEmpty else { return .ignored }
                    editing = false
                    store.removeEmpty(task)
                    return .handled
                }
                .reorderKeys(moveBy)
                .onChange(of: focused) { _, now in if !now && editing { finish(keep: true) } }
        } else {
            Text(task.text).font(TextRole.body.font)
                .foregroundStyle(task.done ? tokens.textMuted : tokens.textPrimary)
                .strikethrough(task.done, color: tokens.textMuted)
                .lineLimit(1).truncationMode(.tail)
                .frame(maxWidth: .infinity, alignment: .leading)
                .contentShape(Rectangle())
                .onTapGesture(perform: edit)
        }
    }

    private func finish(keep: Bool) {
        editing = false
        if keep, draft != task.text { rename(draft) }
    }

    private func edit() { draft = task.text; editing = true; focused = true }
}
