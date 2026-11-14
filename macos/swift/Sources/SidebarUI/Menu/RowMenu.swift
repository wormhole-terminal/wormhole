// A terminal row's context menu, in the order and words of the GPUI app (`sidebar/row_menu.rs`): Rename…, Add terminal
// to new group, Add terminal to group ▸, Remove from group, Split right, Close terminal; Pin to top / Unpin;
// Reveal in Finder, Copy path; Show beside…, Remove from split and the two privacy checks ("Don't keep this terminal's text",
// "Hide from Agents") sit where the GPUI menu has them. Every item is one of the core's actions.

import SwiftUI
import WordsModel
import WorkspaceModel

struct RowMenu: View {
    let terminal: TerminalSummary
    let context: SidebarContext
    @Environment(\.sidebarLook) private var look

    var body: some View {
        let id = terminal.id
        let current = context.group(of: id)
        Button(Words.text("sidebar-menu-rename", fallback: "Rename…")) { context.rename(.terminal(id)) }
        Button(Words.text("sidebar-menu-add-to-new-group", fallback: "Add terminal to new group")) { context.newGroup(id) }
        if !context.groups.isEmpty {
            Menu(Words.text("sidebar-menu-add-to-group", fallback: "Add terminal to group")) {
                ForEach(context.groups) { group in
                    Toggle(group.name, isOn: Binding(get: { group.id == current?.id }, set: { _ in context.send(.addToGroup(id, group.id)) }))
                        .disabled(!context.admits(group.id, id))
                }
            }
        } else {
            Menu(Words.text("sidebar-menu-add-to-group", fallback: "Add terminal to group")) {}.disabled(true)
        }
        if current != nil { Button(Words.text("sidebar-menu-remove-from-group", fallback: "Remove from group")) { context.send(.removeFromGroup(id)) } }
        Button(Words.text("sidebar-menu-split-right", fallback: "Split right")) { context.send(.splitRight(id)) }
        Button(Words.text("sidebar-close-terminal", fallback: "Close terminal")) { context.closeTerminal(id) }
        Divider()
        besideMenu
        if terminal.splitID != nil { Button(Words.text("sidebar-menu-remove-from-split", fallback: "Remove from split")) { context.send(.leaveSplit(id)) } }
        Button(terminal.isPinned ? Words.text("sidebar-unpin", fallback: "Unpin") : Words.text("sidebar-menu-pin-top", fallback: "Pin to top")) { context.send(.togglePin(id)) }
        Toggle(Words.text("sidebar-menu-forget-text", fallback: "Don't keep this terminal's text"), isOn: Binding(get: { terminal.forgetsText }, set: { context.send(.setForgetText(id, $0)) }))
            .disabled(!look.restoresText)
        if look.offersAgentHide || terminal.isAgentHidden {
            Toggle(Words.text("sidebar-menu-hide-from-agents", fallback: "Hide from Agents"), isOn: Binding(get: { terminal.isAgentHidden }, set: { context.send(.setAgentHidden(id, $0)) }))
        }
        Divider()
        let path = terminal.folder.map { HomeFolder.expanding($0) }
        Button(Words.text("files-row-reveal", fallback: "Reveal in Finder")) { if let path { context.hooks.revealFolder(path) } }
            .disabled(path == nil)
        Button(Words.text("title-row-path-copy", fallback: "Copy path")) { if let path { context.hooks.copyPath(path) } }
            .disabled(path == nil)
    }

    /// The terminals that could show next to this one (not already in its split, and the pane has room for): the other joins
    /// it, on its right.
    @ViewBuilder private var besideMenu: some View {
        let others = context.everyTerminal.filter {
            $0.id != terminal.id && ($0.splitID == nil || $0.splitID != terminal.splitID) && context.hasRoom($0.id, terminal.id)
        }
        if others.isEmpty {
            Menu(Words.text("sidebar-menu-show-beside", fallback: "Show beside…")) {}.disabled(true)
        } else {
            Menu(Words.text("sidebar-menu-show-beside", fallback: "Show beside…")) {
                ForEach(others) { other in Button(other.title) { context.send(.showBeside(terminal.id, other: other.id)) } }
            }
        }
    }
}
