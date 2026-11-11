// The small builder every context menu uses: items in order, a separator never first, last or doubled, whatever
// of the items around it were left out (`platform::Menu`'s rule).

import Foundation

struct ContextMenuList {
    private(set) var nodes: [MenuNode] = []

    mutating func add(_ node: MenuNode) { nodes.append(node) }

    mutating func item(
        _ title: String, _ kind: String, subject: Int64? = nil, argument: String? = nil, checked: Bool = false,
        enabled: Bool = true, icon: MenuIcon? = nil, key: KeySpec? = nil
    ) {
        add(.item(MenuItem(
            .context(ContextAction(kind, subject: subject, argument: argument)), title, key: key, checked: checked,
            enabled: enabled, icon: icon)))
    }

    mutating func separator() {
        if let last = nodes.last, last != .separator { nodes.append(.separator) }
    }

    /// A submenu with something in it, or the greyed one when there is nothing to offer.
    mutating func submenu(_ title: String, _ inner: ContextMenuList, empty: Bool? = nil) {
        let none = empty ?? inner.nodes.isEmpty
        add(.submenu(title: title, items: none ? [] : inner.finished, enabled: !none))
    }

    /// The nodes without a trailing separator.
    var finished: [MenuNode] { nodes.last == .separator ? Array(nodes.dropLast()) : nodes }
}
