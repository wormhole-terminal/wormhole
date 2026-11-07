// The Next steps card: its head (the steps still to do, and a + that writes a new one), the steps, and past five
// done ones the "N done" fold that holds them (`view::panel::steps`).

import DesignSystem
import PanelModel
import SwiftUI

struct StepsCard: View {
    @Binding var model: NotesModel
    @State private var writing = false
    @Environment(\.panelMotion) private var motion
    @Environment(\.tokens) private var tokens

    var body: some View {
        let open = model.tasks.remaining
        PanelCard {
            CardBand(
                title: "Next steps", folded: model.tasks.items.isEmpty && !writing ? nil : model.stepsCollapsed,
                count: open > 0 ? CountText.grouped(open) : nil, spoken: "Next steps, \(open) open",
                onToggle: { model.stepsCollapsed.toggle() },
                add: ("Add next step", { model.stepsCollapsed = false; writing = true }))
            if !model.stepsCollapsed {
                ForEach(model.tasks.rows(showDone: model.showDone)) { row in
                    switch row {
                    case let .step(task):
                        TaskRowView(
                            task: task, toggle: { model.tasks.toggle(task.id) },
                            rename: { model.tasks.rename(task.id, to: $0) }, send: {},
                            delete: { model.tasks.delete(task.id) })
                            .transition(motion.row)
                    case let .doneFold(count):
                        DoneFold(count: count, open: model.showDone) { model.showDone.toggle() }.transition(motion.row)
                    }
                }
                if writing {
                    TaskDraftRow(add: { _ = model.tasks.add($0) }, discard: { writing = false }).transition(motion.row)
                }
            }
        }
        .animation(motion.list, value: model.tasks.rows(showDone: model.showDone))
        .animation(motion.fold, value: model.stepsCollapsed)
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
                Text("\(CountText.grouped(count)) done").font(TextRole.body.font).foregroundStyle(tokens.textMuted)
                Spacer(minLength: 0)
            }
            .padding(.horizontal, PanelMetrics.inset).frame(minHeight: Metric.row)
        }
    }
}
