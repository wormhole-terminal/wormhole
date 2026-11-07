// A terminal row's context menu, in the order and words of the GPUI app (`sidebar/row_menu.rs`): Rename…, Add terminal
// to new group, Add terminal to group ▸, Remove from group, Split right, Close terminal; Pin to top / Unpin;
// Reveal in Finder, Copy path. Every item is a real action of the window's model. "Show beside…", "Remove from
// split" and the two privacy items wait for the model to carry them.

import AppKit
import SwiftUI
import WorkspaceModel

struct RowMenu: View {
    let terminal: TerminalSummary
    let context: SidebarContext

    var body: some View {
        let id = terminal.id
        let current = context.group(of: id)
        Button("Rename…") { context.rename(id) }
        Button("Add terminal to new group") {
            context.send(.newGroup(""))
            context.send(.addToGroup(id, context.nextGroupID))
        }
        if !context.groups.isEmpty {
            Menu("Add terminal to group") {
                ForEach(context.groups) { group in
                    Toggle(group.name, isOn: Binding(get: { group.id == current?.id }, set: { _ in context.send(.addToGroup(id, group.id)) }))
                }
            }
        } else {
            Menu("Add terminal to group") {}.disabled(true)
        }
        if current != nil { Button("Remove from group") { context.send(.moveTerminal(id, to: .top(before: nil))) } }
        Button("Split right") { context.send(.splitRight(id)) }
        Button("Close terminal") { context.send(.close(id)) }
        Divider()
        Button(terminal.isPinned ? "Unpin" : "Pin to top") { context.send(.togglePin(id)) }
        Divider()
        let path = terminal.folder.map { NSString(string: $0).expandingTildeInPath }
        Button("Reveal in Finder") { if let path { NSWorkspace.shared.activateFileViewerSelecting([URL(fileURLWithPath: path)]) } }
            .disabled(path == nil)
        Button("Copy path") {
            if let path {
                NSPasteboard.general.clearContents()
                NSPasteboard.general.setString(path, forType: .string)
            }
        }
        .disabled(path == nil)
    }
}
