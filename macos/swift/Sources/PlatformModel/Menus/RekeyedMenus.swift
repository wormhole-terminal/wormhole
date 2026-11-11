// The menu bar with the keys the person has changed: core's menus carry the table's keys, and the key that jumps to the waiting
// terminal is the person's to record (`Keymap.replacingKeys`), so an item shows the key `Keymap.shared` has now.

public struct RekeyedMenus: MenuProviding {
    public let base: any MenuProviding

    public init(base: any MenuProviding) { self.base = base }

    public func menus(for state: MenuState) -> [Menu] {
        let keymap = Keymap.shared
        return base.menus(for: state).map { Menu(title: $0.title, items: $0.items.map { Self.rekey($0, keymap) }) }
    }

    private static func rekey(_ node: MenuNode, _ keymap: Keymap) -> MenuNode {
        switch node {
        case .item(var item):
            if case .action(let id) = item.command, id == .goToWaiting { item.key = keymap.key(id) }
            return .item(item)
        case let .submenu(title, items, enabled, trailing):
            return .submenu(title: title, items: items.map { rekey($0, keymap) }, enabled: enabled, trailing: trailing)
        default: return node
        }
    }
}
