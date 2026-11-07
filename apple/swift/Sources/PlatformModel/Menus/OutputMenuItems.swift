// Export Text As… and Print… in the Shell menu, where Terminal.app has them. The core keeps ⌘S and ⌘P for them
// ("reserved") but lists them in no menu yet, so this adds them after Close Window; the keys, titles and the
// actions are the keymap's own, and an action with no handler is greyed like any other.

public struct OutputMenuItems: MenuProviding {
    private let base: any MenuProviding
    private let keymap: Keymap

    public init(base: any MenuProviding, keymap: Keymap = .shared) {
        self.base = base
        self.keymap = keymap
    }

    public func menus(for state: MenuState) -> [Menu] {
        base.menus(for: state).map { menu in
            guard menu.title == "Shell" else { return menu }
            let ids: [ActionID] = ["exportText", "print"]
            let items = ids.compactMap { id in
                keymap.entry(id).map { MenuNode.item(MenuItem(.action(id), $0.title(on: true), key: $0.key)) }
            }
            return Menu(title: menu.title, items: menu.items + [.separator] + items)
        }
    }
}
