// The menu bar for a window's state (`keymap::menus::menus`): the layout's slots become items, an action that
// is not live (or is the system's, or whose feature is off) is left out, and so is a separator it would leave
// dangling. This is the build without the Rust core: linked, `PlatformBridge.makeMenus()` answers the same
// question from core's own `keymap::menus::menus` and a test keeps the two equal.

import Foundation

/// Where the menu bar's values come from for a window's state.
public protocol MenuProviding: Sendable {
    /// The menu bar, left to right.
    func menus(for state: MenuState) -> [Menu]
}

public struct MenuBuilder: MenuProviding {
    public let keymap: Keymap

    public init(keymap: Keymap = .shared) { self.keymap = keymap }

    /// The menu bar, left to right.
    public func menus(for state: MenuState) -> [Menu] {
        keymap.layout
            .filter { $0.when != "debug" || state.debug }
            .map { Menu(title: $0.title, items: tidy(fill($0.slots, state))) }
    }

    func fill(_ slots: [MenuLayout.Slot], _ state: MenuState) -> [MenuNode] {
        slots.flatMap { slot -> [MenuNode] in
            switch slot {
            case .action(let id): actionItem(id, state).map { [.item($0)] } ?? []
            case .separator: [.separator]
            case .services: [.services]
            case .submenu(let title, let slots):
                { inner in inner.isEmpty ? [] : [.submenu(title: title, items: inner)] }(tidy(fill(slots, state)))
            case .dynamic(let part): dynamic(part, state)
            }
        }
    }

    func actionItem(_ id: ActionID, _ state: MenuState) -> MenuItem? {
        guard let entry = keymap.entry(id), entry.stage == .now, !(entry.debugOnly && !state.debug),
            !(entry.persistentOnly && !state.persistent), !state.hidden.contains(id)
        else { return nil }
        // In a split ⌘W closes the focused pane, so its item says so.
        var title = id == .closeTerminal && state.split ? "Close Pane" : entry.title(on: !state.off.contains(id))
        if id == .undo, let what = state.undo { title += " \(what)" }
        if id == .redo, let what = state.redo { title += " \(what)" }
        return MenuItem(
            .action(id), title, key: entry.key, checked: state.checked.contains(id),
            enabled: !state.disabled.contains(id))
    }

    /// No separator at either end of a menu or next to another one.
    func tidy(_ items: [MenuNode]) -> [MenuNode] {
        var out: [MenuNode] = []
        for item in items {
            if item == .separator, out.last.map({ $0 == .separator }) ?? true { continue }
            out.append(item)
        }
        if out.last == .separator { out.removeLast() }
        return out
    }
}
