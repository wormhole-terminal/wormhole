// A group's menu (`sidebar/group_popover.rs` `group_menu`), native: a right-click on the group's head or on its card in the
// tab bar's switcher asks for it, and ⌃↩ on the head opens it under the head (`group_head.rs` `group_key_menu`). Its items
// are `PlatformModel.GroupMenu`'s, the nine colours a row of dots in the hues the head's own dot has (from the theme's
// palette, not from a token); "Other colour…" opens the system's Colours window (the popover in a test instance, which shows no system panel). A test instance keeps a keyed menu for `menu` and
// `menu-pick`, as the row's.

import AppKit
import DesignSystem
import PlatformModel
import PlatformUI
import SidebarUI
import WindowUI
import WorkspaceModel

extension AppController {
    func openGroupMenu(_ id: GroupID, at frame: CGRect, in session: WindowSession) {
        guard let menu = groupMenu(id, in: session) else { return }
        guard policy.touchesSystem, let content = session.window?.contentView else { return injector.keep(menu) }
        menu.popUp(positioning: nil, at: NSPoint(x: frame.minX + 24, y: content.bounds.height - frame.maxY + 4), in: content)
    }

    /// The group's menu as it is now, with the nine colours as a row of dots; a right-click on its head or on its card in
    /// the tab bar's switcher asks for it too.
    func groupMenu(_ id: GroupID, in session: WindowSession) -> NSMenu? {
        guard let head = session.hub.state.groups.first(where: { $0.id == id }) else { return nil }
        var facts = GroupMenuFacts(group: Int64(id))
        facts.pinned = head.isPinned
        facts.custom = head.customColor != nil
        facts.coloured = head.color != nil
        facts.colours = GroupColor.allCases.map { .init(name: $0.rawValue.capitalized, token: Self.dotToken + $0.rawValue) } // text: a colour's name for VoiceOver, as the popover's swatches say it
        facts.chosen = head.color.flatMap(GroupColor.allCases.firstIndex(of:))
        facts.folder = head.folder
        facts.onlyInFolder = head.onlyInFolder
        // The dots are the colours as the head's dot shows them (hues from the theme's palette), which no theme token names.
        let themed = MenuStyle.themed()
        let style = MenuStyle { [themes] token in
            guard token.hasPrefix(Self.dotToken) else { return themed.color(token) }
            return GroupColor(rawValue: String(token.dropFirst(Self.dotToken.count)))?.mark(themes.tokens).nsColor
        }
        return NSMenuFactory(dispatcher: contextMenus.dispatcher, style: style).menu(title: "", GroupMenu.nodes(facts))
    }

    private static let dotToken = "group."

    /// A choice from a group's menu: the action of the item of the same name in the SwiftUI menu.
    func runGroup(_ kind: GroupMenu.Kind, _ action: ContextAction) {
        guard let subject = action.subject, case let id = GroupID(subject),
              let session = sessions.first(where: { $0.hub.state.groups.contains { $0.id == id } }) else { return }
        let hub = session.hub
        switch kind {
        case .rename: hub.sidebarLink.ask(.rename(.group(id)))
        case .colour:
            let colours = GroupColor.allCases
            if let at = action.argument.flatMap({ Int($0) }), colours.indices.contains(at) { hub.send(.setGroupColor(id, colours[at])) }
        case .otherColour: pickColour(of: id, in: hub)
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

    /// "Other colour…": the Colours window on the colour the group shows (its own, else its named one, else mid grey:
    /// `group_popover.rs` `START`), the group following each pick.
    private func pickColour(of id: GroupID, in hub: WindowHub) {
        guard policy.touchesSystem else { return hub.sidebarLink.ask(.popover(id)) }
        let head = hub.state.groups.first { $0.id == id }
        let start = head?.customColor.flatMap(NSColor.init(groupHex:)) ?? head?.color?.mark(themes.tokens).nsColor ?? NSColor(srgbRed: 0.5, green: 0.5, blue: 0.5, alpha: 1)
        GroupColourPanel.open(start: start) { [weak hub] red, green, blue in
            hub?.send(.setGroupCustomColor(id, red: red, green: green, blue: blue))
        }
    }
}
