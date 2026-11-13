// ⌃↩ on a sidebar row opens the row's menu (`sidebar/row_menu.rs`). A SwiftUI `.contextMenu` cannot be opened from code, so the keys
// get the same items as a native menu (`PlatformModel.RowMenu`, which is what GPUI's menu is too) under the row; choosing one runs
// the action the SwiftUI menu's item would. A test instance never puts a menu on the screen (it would hold the app in AppKit's
// tracking loop): it keeps it for `menu` and `menu-pick`.

import AppKit
import DesignSystem
import PlatformModel
import WindowUI
import WorkspaceModel

extension AppController {
    func openRowMenu(_ id: TerminalID, at frame: CGRect, in session: WindowSession) {
        let hub = session.hub
        guard let terminal = hub.state.all.first(where: { $0.id == id }) else { return }
        var facts = RowMenuFacts(id: Int64(id))
        facts.pinned = terminal.isPinned
        facts.forgetText = terminal.forgetsText
        // The window's own profile's switches.
        let sidebar = session.dress.look.sidebar
        facts.agentHide = sidebar.offersAgentHide || terminal.isAgentHidden ? terminal.isAgentHidden : nil
        facts.restoring = sidebar.restoresText
        facts.inSplit = terminal.splitID != nil
        facts.cwd = terminal.folder.map { ($0 as NSString).expandingTildeInPath }
        let current = hub.state.sections.first { $0.rows.contains { $0.id == id } }?.head?.id
        facts.groups = hub.state.groups.map {
            .init(id: Int64($0.id), name: $0.name, current: $0.id == current, allowed: hub.store.groupAdmits($0.id, terminal: id))
        }
        facts.beside = hub.state.all.filter {
            $0.id != id && ($0.splitID == nil || $0.splitID != terminal.splitID)
                && hub.store.paneDropRefusal(dragged: $0.id, target: id, zone: .right) == nil
        }.map { .init(id: Int64($0.id), name: $0.title) }
        let menu = contextMenus.menu(RowMenu.nodes(facts))
        guard policy.touchesSystem, let content = session.window?.contentView else { return injector.keep(menu) }
        // The frame is in the window's content from its top left; AppKit's menu opens at a point of the view, from its bottom left.
        menu.popUp(positioning: nil, at: NSPoint(x: frame.minX + 24, y: content.bounds.height - frame.maxY + 4), in: content)
    }

    /// A choice from a row's menu: the action of the item of the same name in the SwiftUI menu.
    func runRow(_ kind: RowMenu.Kind, _ action: ContextAction) {
        guard let subject = action.subject, case let id = TerminalID(subject), let session = session(holding: id) else { return }
        let hub = session.hub
        let terminal = hub.state.all.first { $0.id == id }
        let other = action.argument.flatMap { Int($0) }
        switch kind {
        case .rename: hub.sidebarLink.ask(.rename(.terminal(id)))
        case .newGroup:
            hub.send(.newGroup([id]))
            if let head = hub.state.sections.first(where: { $0.rows.contains { $0.id == id } })?.head { hub.sidebarLink.ask(.rename(.group(head.id))) }
        case .intoGroup: if let group = other { hub.send(.addToGroup(id, GroupID(group))) }
        case .removeFromGroup: hub.send(.removeFromGroup(id))
        case .splitRight: hub.send(.splitRight(id))
        case .close: hub.actions.closeTerminal(id)
        case .showBeside: if let other { hub.send(.showBeside(id, other: other)) }
        case .removeFromSplit: hub.send(.leaveSplit(id))
        case .togglePin: hub.send(.togglePin(id))
        case .toggleForgetText: hub.send(.setForgetText(id, !(terminal?.forgetsText ?? false)))
        case .toggleAgentHidden: hub.send(.setAgentHidden(id, !(terminal?.isAgentHidden ?? false)))
        case .revealInFinder: if let path = terminal?.folder { hub.revealFolder((path as NSString).expandingTildeInPath) }
        case .copyPath: if let path = terminal?.folder { hub.copyFolder((path as NSString).expandingTildeInPath) }
        }
    }
}
