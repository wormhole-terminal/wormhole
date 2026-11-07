// The parts of the menu bar that follow the window in front: recent folders, the project's actions, the shells,
// the profiles and the terminals 1–9.

import Foundation

extension MenuBuilder {
    func dynamic(_ part: MenuLayout.Dynamic, _ state: MenuState) -> [MenuNode] {
        switch part {
        case .recentFolders:
            [list("Open Recent", state.recent.enumerated().map { .item(MenuItem(.openRecent($0), $1)) }, "No Recent Folders")]
        case .projectActions:
            [list("Project Actions", state.projectActions.map { .item(MenuItem(.runProjectAction($0.id), $0.title)) }, "No Project Actions")]
        case .newTerminalWith:
            [list("New Terminal With", state.shells.enumerated().map { .item(MenuItem(.newTerminalWith($0), $1)) }, "No Shells")]
        case .profiles: [profiles(state)]
        case .terminals:
            state.terminals.prefix(9).enumerated().map { index, name in
                let id = ActionID.terminal(index + 1)
                return .item(MenuItem(.action(id), name, key: keymap.key(id), checked: state.currentTerminal == index))
            }
        }
    }

    /// A submenu of what the window has; with nothing, the greyed `empty` in it.
    private func list(_ title: String, _ items: [MenuNode], _ empty: String) -> MenuNode {
        .submenu(title: title, items: items.isEmpty ? [.placeholder(empty)] : items)
    }

    /// Profile ▸ the profiles (the shown one checked), what opens a window in one or moves this window to one
    /// (only with another profile to pick), then New Profile… and Manage Profiles….
    private func profiles(_ state: MenuState) -> MenuNode {
        func pick(_ command: (Int) -> MenuCommand, skipping: Int?) -> [MenuNode] {
            state.profiles.enumerated().filter { $0.offset != skipping }.map { .item(MenuItem(command($0.offset), $0.element)) }
        }
        var items: [MenuNode] = state.profiles.enumerated().map {
            .item(MenuItem(.switchProfile($0.offset), $0.element, checked: state.currentProfile == $0.offset))
        }
        if state.profiles.count > 1 {
            items += [
                .separator,
                .submenu(title: "Open Window in", items: pick(MenuCommand.openInProfile, skipping: nil)),
                .submenu(
                    title: "Move Window to", items: pick(MenuCommand.moveToProfile, skipping: state.windowProfile),
                    enabled: state.windowProfile != nil),
            ]
        }
        items.append(.separator)
        items += [ActionID.newProfile, .manageProfiles].compactMap { actionItem($0, state).map(MenuNode.item) }
        return .submenu(title: "Profile", items: tidy(items))
    }
}
