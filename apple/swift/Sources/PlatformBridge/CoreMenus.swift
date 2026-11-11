// The menu bar for a window's state, from core (`keymap::menus::menus`): the layout, the flipping titles, what is
// hidden or disabled, the numbered terminals. The window's facts go in as `MenuStateRecord`; the values come back
// as `PlatformModel`'s `Menu`.

import PlatformModel
import WormholeBindings

struct CoreMenus: MenuProviding {
    func menus(for state: MenuState) -> [Menu] {
        keymapMenus(state: state.record).map { Menu(title: $0.title, items: $0.items.map(MenuNode.init)) }
    }
}

extension MenuState {
    var record: MenuStateRecord {
        MenuStateRecord(
            off: off.map(\.rawValue), checked: checked.map(\.rawValue), disabled: disabled.map(\.rawValue),
            hidden: hidden.map(\.rawValue), terminals: terminals, currentTerminal: currentTerminal.map(UInt32.init),
            recent: recent, projectActions: projectActions.map { MenuProjectActionRecord(id: $0.id, title: $0.title) },
            shells: shells, profiles: profiles, currentProfile: currentProfile.map(UInt32.init),
            windowProfile: windowProfile.map(UInt32.init), split: split, undo: undo, redo: redo, debug: debug,
            persistent: persistent)
    }
}

extension MenuCommand {
    init(_ record: MenuCommandRecord) {
        self =
            switch record {
            case .action(let id): .action(ActionID(rawValue: id))
            case .openRecent(let index): .openRecent(Int(index))
            case .runProjectAction(let id): .runProjectAction(id)
            case .newTerminalWith(let index): .newTerminalWith(Int(index))
            case .switchProfile(let index): .switchProfile(Int(index))
            case .openInProfile(let index): .openInProfile(Int(index))
            case .moveToProfile(let index): .moveToProfile(Int(index))
            }
    }
}

extension MenuNode {
    init(_ record: MenuItemRecord) {
        switch record.kind {
        case .action:
            // An action item always carries its command; one that does not is not drawn.
            guard let command = record.command else { self = .separator; return }
            self = .item(
                MenuItem(
                    MenuCommand(command), record.title, key: record.key.map(KeySpec.init), checked: record.checked,
                    enabled: record.enabled))
        case .separator: self = .separator
        case .services: self = .services
        case .submenu:
            self = .submenu(title: record.title, items: record.items.map(MenuNode.init), enabled: record.enabled)
        case .placeholder: self = .placeholder(record.title)
        }
    }
}
