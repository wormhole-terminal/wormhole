// ⌃↩ on a sidebar group's head opens the group's menu under it (`sidebar/group_head.rs` `group_key_menu`): the items of
// `PlatformModel.GroupMenu`, and choosing one does what the item of the same name in the SwiftUI menu does. The colour dots
// are the SwiftUI menu's and popover's (their hues come from the theme's palette, not from a token); "Other colour…" opens
// the popover. A test instance keeps the menu for `menu` and `menu-pick`, as the row's.

import AppKit
import PlatformModel
import SidebarUI
import WindowUI
import WorkspaceModel

extension AppController {
    func openGroupMenu(_ id: GroupID, at frame: CGRect, in session: WindowSession) {
        guard let menu = groupMenu(id, in: session) else { return }
        guard policy.touchesSystem, let content = session.window?.contentView else { return injector.keep(menu) }
        menu.popUp(positioning: nil, at: NSPoint(x: frame.minX + 24, y: content.bounds.height - frame.maxY + 4), in: content)
    }

    /// The group's menu as it is now; a right-click on its card in the tab bar's switcher asks for it too.
    func groupMenu(_ id: GroupID, in session: WindowSession) -> NSMenu? {
        guard let head = session.hub.state.groups.first(where: { $0.id == id }) else { return nil }
        var facts = GroupMenuFacts(group: Int64(id))
        facts.pinned = head.isPinned
        facts.custom = head.customColor != nil
        facts.coloured = head.color != nil
        facts.folder = head.folder
        facts.onlyInFolder = head.onlyInFolder
        return contextMenus.menu(GroupMenu.nodes(facts))
    }

    /// A choice from a group's menu: the action of the item of the same name in the SwiftUI menu.
    func runGroup(_ kind: GroupMenu.Kind, _ action: ContextAction) {
        guard let subject = action.subject, case let id = GroupID(subject),
              let session = sessions.first(where: { $0.hub.state.groups.contains { $0.id == id } }) else { return }
        let hub = session.hub
        switch kind {
        case .rename: hub.sidebarLink.ask(.rename(.group(id)))
        case .colour: break
        case .otherColour: hub.sidebarLink.ask(.popover(id))
        case .noColour: hub.send(.setGroupColor(id, nil))
        case .setFolder: GroupFolderPanel.choose(for: id, send: hub.send)
        case .clearFolder: hub.send(.setGroupFolder(id, nil))
        case .onlyInFolder: hub.send(.setOnlyInFolder(id, !(hub.state.groups.first { $0.id == id }?.onlyInFolder ?? false)))
        case .newTerminal: hub.sidebarLink.ask(.newInGroup(id))
        case .ungroup: hub.send(.ungroup(id))
        case .close: hub.actions.closeGroup(id)
        case .togglePin: hub.send(.setGroupPinned(id, !(hub.state.groups.first { $0.id == id }?.isPinned ?? false)))
        case .moveToWindow: hub.send(.moveGroupToWindow(id))
        }
    }
}
