// The Notes tab: free text for the terminal, then its next steps. Flat, with hairlines between the two.

import DesignSystem
import PanelModel
import SwiftUI

public struct NotesView: View {
    @Binding var model: NotesModel
    @Environment(\.panelMotion) private var motion

    public init(model: Binding<NotesModel>) { _model = model }

    public var body: some View {
        ScrollView {
            VStack(spacing: 0) {
                SectionHeader(title: "Notes")
                editor
                Rectangle().fill(Palette.divider).frame(height: 1).padding(.top, Space._2)
                steps
            }
        }
        .scrollIndicators(.automatic)
    }

    private var editor: some View {
        TextEditor(text: $model.text)
            .font(.callout)
            .scrollContentBackground(.hidden)
            .scrollDisabled(true)
            .frame(minHeight: 96)
            .padding(.horizontal, Space._3 - 5)
            .overlay(alignment: .topLeading) {
                if model.text.isEmpty {
                    Text("Notes for this terminal").font(.callout).foregroundStyle(.tertiary)
                        .padding(.horizontal, Space._3).padding(.top, 0).allowsHitTesting(false)
                }
            }
            .accessibilityLabel("Notes")
    }

    private var steps: some View {
        VStack(spacing: 0) {
            SectionHeader(title: "Next steps", count: model.tasks.remaining) {
                if model.tasks.items.contains(where: \.done) {
                    Button("Clear done") { withAnimation(motion.list) { model.tasks.clearDone() } }
                        .buttonStyle(.plain).font(.subheadline).foregroundStyle(.secondary)
                        .transition(.opacity)
                }
            }
            .animation(motion.list, value: model.tasks.items.contains(where: \.done))
            ForEach(model.tasks.items) { task in
                TaskRow(
                    task: task,
                    toggle: { withAnimation(motion.tick) { model.tasks.toggle(task.id) } },
                    delete: { withAnimation(motion.list) { model.tasks.delete(task.id) } }
                )
                .transition(motion.row)
            }
            AddTaskField { text in withAnimation(motion.list) { _ = model.tasks.add(text) } }
        }
    }
}
