// A terminal row's context menu, in the order and words of the GPUI app (`sidebar/row_menu.rs`): Rename…, Add terminal
// to new group, Add terminal to group ▸, Remove from group, Split right, Close terminal; Pin to top / Unpin;
// Reveal in Finder, Copy path; Show beside…, Remove from split and the two privacy checks ("Don't keep this terminal's text",
// "Hide from Agents") sit where the GPUI menu has them. Every item is one of the core's actions.

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
        Button("Add terminal to new group") { context.send(.newGroup([id])) }
        if !context.groups.isEmpty {
            Menu("Add terminal to group") {
                ForEach(context.groups) { group in
                    Toggle(group.name, isOn: Binding(get: { group.id == current?.id }, set: { _ in context.send(.addToGroup(id, group.id)) }))
                }
            }
        } else {
            Menu("Add terminal to group") {}.disabled(true)
        }
        if current != nil { Button("Remove from group") { context.send(.removeFromGroup(id)) } }
        Button("Split right") { context.send(.splitRight(id)) }
        Button("Close terminal") { context.send(.close(id)) }
        Divider()
        besideMenu
        if terminal.splitID != nil { Button("Remove from split") { context.send(.leaveSplit(id)) } }
        Button(terminal.isPinned ? "Unpin" : "Pin to top") { context.send(.togglePin(id)) }
        Toggle("Don't keep this terminal's text", isOn: Binding(get: { terminal.forgetsText }, set: { context.send(.setForgetText(id, $0)) }))
        Toggle("Hide from Agents", isOn: Binding(get: { terminal.isAgentHidden }, set: { context.send(.setAgentHidden(id, $0)) }))
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

    /// The terminals that could show next to this one (not already in its split): the other joins it, on its right.
    @ViewBuilder private var besideMenu: some View {
        let others = context.everyTerminal.filter { $0.id != terminal.id && ($0.splitID == nil || $0.splitID != terminal.splitID) }
        if others.isEmpty {
            Menu("Show beside…") {}.disabled(true)
        } else {
            Menu("Show beside…") {
                ForEach(others) { other in Button(other.title) { context.send(.showBeside(terminal.id, other: other.id)) } }
            }
        }
    }
}
