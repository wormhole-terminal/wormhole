// A row's and a group head's context menus, in the order and words of the GPUI app (`sidebar/row_menu.rs`).
// Items the model has no action for yet are shown dimmed, so the menu is the real one and nothing is promised.

import AppKit
import SwiftUI
import WorkspaceModel

enum SidebarMenu {
    @MainActor @ViewBuilder static func row(_ terminal: TerminalSummary, send: @escaping @MainActor (WorkspaceAction) -> Void) -> some View {
        Button("Rename…") {}.disabled(true)
        Button("Add terminal to new group") {}.disabled(true)
        Button("Split right") {}.disabled(true)
        Button("Close terminal") { send(.close(terminal.id)) }
        Divider()
        Button(terminal.isPinned ? "Unpin" : "Pin to top") { send(.togglePin(terminal.id)) }
        Divider()
        let path = folder(terminal)
        Button("Reveal in Finder") { if let path { NSWorkspace.shared.activateFileViewerSelecting([URL(fileURLWithPath: path)]) } }.disabled(path == nil)
        Button("Copy path") {
            if let path {
                NSPasteboard.general.clearContents()
                NSPasteboard.general.setString(path, forType: .string)
            }
        }.disabled(path == nil)
    }

    @MainActor @ViewBuilder static func head(_ section: SidebarSection, send: @escaping @MainActor (WorkspaceAction) -> Void) -> some View {
        Button(section.isCollapsed ? "Open group" : "Fold group") { send(.toggleGroup(section.id)) }
    }

    /// The terminal's folder when its subtitle is one (`~/…` or absolute).
    private static func folder(_ terminal: TerminalSummary) -> String? {
        guard let subtitle = terminal.subtitle else { return nil }
        if subtitle.hasPrefix("/") { return subtitle }
        if subtitle.hasPrefix("~") { return NSString(string: subtitle).expandingTildeInPath }
        return nil
    }
}
