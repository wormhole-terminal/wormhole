// One changed file: status letter, name with its folder dimmed after it, and the +/- counts. Used by the
// working tree list and by a commit's files.

import DesignSystem
import PanelModel
import SwiftUI

struct FileRow: View {
    let file: ChangedFile
    var selected = false
    var space: Namespace.ID
    @State private var hovered = false
    @Environment(\.panelCopy) private var copy

    var body: some View {
        HStack(spacing: Space._2) {
            StatusBadge(status: file.status)
            // The name never truncates before its folder does: the folder is context, the name is identity.
            HStack(spacing: Space._1) {
                Text(file.name).font(.callout).lineLimit(1).layoutPriority(1)
                if !file.folder.isEmpty {
                    Text(file.folder).font(.subheadline).foregroundStyle(.tertiary).lineLimit(1)
                        .truncationMode(.head)
                }
            }
            Spacer(minLength: 0)
            ChangeCounts(added: file.added, removed: file.removed)
        }
        .padding(.horizontal, Space._2)
        .frame(height: 28)
        .contentShape(Rectangle())
        .selectionBackdrop(selected: selected, hovered: hovered, in: space)
        .onHover { hovered = $0 }
        .contextMenu {
            Button("Copy Path") { copy(file.path) }
            Button("Copy File Name") { copy(file.name) }
        }
        .accessibilityElement(children: .ignore)
        .accessibilityLabel(label)
        .accessibilityAddTraits(selected ? [.isButton, .isSelected] : .isButton)
    }

    private var label: String {
        var parts = ["\(file.status.spoken), \(file.name)"]
        if !file.folder.isEmpty { parts.append("in \(file.folder)") }
        if file.added > 0 { parts.append("\(file.added) added") }
        if file.removed > 0 { parts.append("\(file.removed) removed") }
        return parts.joined(separator: ", ")
    }
}
