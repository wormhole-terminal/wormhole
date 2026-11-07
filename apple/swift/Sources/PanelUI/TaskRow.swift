// One next step: tick box, text, and a delete button that shows on hover. Click anywhere on the row to tick it.

import DesignSystem
import PanelModel
import SwiftUI

struct TaskRow: View {
    let task: PanelTask
    let toggle: () -> Void
    let delete: () -> Void
    @State private var hovered = false
    @Environment(\.panelCopy) private var copy
    @Environment(\.panelMotion) private var motion

    var body: some View {
        HStack(alignment: .firstTextBaseline, spacing: Space._2) {
            TaskCheck(done: task.done).alignmentGuide(.firstTextBaseline) { $0[VerticalAlignment.center] + 4 }
            Text(task.text)
                .font(.callout)
                .strikethrough(task.done)
                .foregroundStyle(task.done ? .tertiary : .primary)
                .animation(motion.tick, value: task.done)
                .frame(maxWidth: .infinity, alignment: .leading)
            Button(action: delete) {
                Image(systemName: "xmark").font(.system(size: 10, weight: .semibold))
            }
            .buttonStyle(.plain)
            .foregroundStyle(.secondary)
            .opacity(hovered ? 1 : 0)
            .accessibilityLabel("Delete \(task.text)")
        }
        .padding(.horizontal, Space._3)
        .padding(.vertical, Space._1 + 2)
        .contentShape(Rectangle())
        .onTapGesture(perform: toggle)
        .onHover { hovered = $0 }
        .animation(motion.select, value: hovered)
        .contextMenu {
            Button(task.done ? "Mark as Not Done" : "Mark as Done", action: toggle)
            Button("Copy Text") { copy(task.text) }
            Divider()
            Button("Delete", role: .destructive, action: delete)
        }
        .accessibilityElement(children: .ignore)
        .accessibilityLabel(task.text)
        .accessibilityValue(task.done ? "Done" : "Not done")
        .accessibilityAddTraits(.isButton)
        .accessibilityAction(named: "Delete", delete)
    }
}
