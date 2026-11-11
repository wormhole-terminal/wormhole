// The Next steps card: its head (the steps still to do, and a + that writes a new one), the steps, and past five
// done ones the "N done" fold that holds them (`view::panel::steps`).

import DesignSystem
import PanelModel
import SwiftUI

struct StepsCard: View {
    @Bindable var store: NotesStore
    @State private var dragging: UUID?
    @Environment(\.panelMotion) private var motion
    @Environment(\.tokens) private var tokens

    private var model: NotesModel {
        get { store.model }
        nonmutating set { store.model = newValue }
    }

    var body: some View {
        let open = model.tasks.remaining
        PanelCard {
            CardBand(
                title: panelText("steps-title", "Next steps"), folded: model.tasks.items.isEmpty && !store.drafting ? nil : model.stepsCollapsed,
                count: open > 0 ? CountText.grouped(open) : nil, spoken: panelText("steps-spoken", ["count": .whole(open)], "Next steps, \(String(open)) open"),
                onToggle: { model.stepsCollapsed.toggle() },
                add: (panelText("steps-add", "Add next step"), { model.stepsCollapsed = false; (store.draftAfter, store.drafting) = (nil, true) }))
                .walkRow("band-steps")
            if !model.stepsCollapsed {
                ForEach(model.tasks.rows(showDone: model.showDone)) { row in
                    switch row {
                    case let .step(task):
                        let ids = model.tasks.items.map(\.id)
                        TaskRowView(
                            task: task, store: store, toggle: { model.tasks.toggle(task.id) },
                            rename: { model.tasks.rename(task.id, to: $0) }, send: { store.send(task) },
                            delete: { model.tasks.delete(task.id) },
                            moveBy: { delta in
                                if let from = ids.firstIndex(of: task.id), let to = Reorder.step(from, delta, count: ids.count) { store.moveTask(from: from, to: to) }
                            })
                            .reorderable(id: task.id, in: ids, dragging: $dragging, move: store.moveTask)
                            .walkRow("step-" + task.id.uuidString)
                            .transition(motion.row)
                        if store.drafting, store.draftAfter == task.id { draft }
                    case let .doneFold(count):
                        DoneFold(count: count, open: model.showDone) { model.showDone.toggle() }.walkRow("done-fold").transition(motion.row)
                    }
                }
                if store.drafting, store.draftAfter == nil { draft }
            }
        }
        .animation(motion.list, value: model.tasks.rows(showDone: model.showDone))
        .animation(motion.fold, value: model.stepsCollapsed)
    }
}

extension StepsCard {
    /// The step being written, where ↵ left it: under the step it was opened after, or at the end. Each new one is
    /// a new row (its id is its place), which takes the keyboard on its own.
    fileprivate var draft: some View {
        let after = store.draftAfter
        return TaskDraftRow(
            add: { text in
                if let made = store.model.tasks.add(text, after: after) { store.draftAfter = made.id }
            },
            discard: { store.drafting = false },
            leave: store.leave
        )
        .id(store.draftAfter)
        .transition(motion.row)
    }
}

private struct DoneFold: View {
    let count: Int
    let open: Bool
    let action: () -> Void
    @Environment(\.tokens) private var tokens

    var body: some View {
        PanelRow(action: action) { _ in
            HStack(spacing: PanelMetrics.gap) {
                Disclosure(collapsed: !open)
                Text(panelText("steps-done", ["count": .text(CountText.grouped(count))], "\(CountText.grouped(count)) done")).font(TextRole.body.font).foregroundStyle(tokens.textMuted)
                Spacer(minLength: 0)
            }
            .padding(.horizontal, PanelMetrics.inset).frame(minHeight: Metric.row)
        }
    }
}
