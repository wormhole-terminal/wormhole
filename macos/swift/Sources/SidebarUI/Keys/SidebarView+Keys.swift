// The list under the keys: ↑ ↓ ← → move the cursor and fold or open groups, ⌥↑ ⌥↓ go to the ends, letters jump by name,
// Return opens a terminal and gives it the keyboard, Space shows it and keeps the keyboard in the list (browsing); on a
// group head either folds it (`SidebarWalk` decides each move). ⌃↩ opens the menu of the row or head the keys are on.
// Esc ends a drag in flight.

import SwiftUI
import WorkspaceModel

extension SidebarView {
    func handle(_ press: KeyPress, _ walk: SidebarWalk) -> KeyPress.Result {
        if press.key == .escape, drag != nil {
            cancelDrag()
            return .handled
        }
        keysLead = true
        let at = walk.index(of: cursor)
        // ⌃↩: the menu of the row or head the keys are on (`view::focus`: the list's menu key).
        if press.key == .return, press.modifiers.contains(.control), let at {
            switch walk.items[at].stop {
            case .row(let id): link.rowMenu(id, rowFrames.frames[id] ?? .zero)
            case .group(let id): link.groupMenu(id, rowFrames.heads[id] ?? .zero)
            }
            return .handled
        }
        let key: ListKey? = switch press.key {
        case .upArrow: press.modifiers.contains(.option) ? .first : .up
        case .downArrow: press.modifiers.contains(.option) ? .last : .down
        case .leftArrow: .left
        case .rightArrow: .right
        default: nil
        }
        if let key {
            switch walk.move(from: at, key) {
            case .to(let index): cursor = walk.items[index].stop
            case .open(let index), .fold(let index): if case .group(let id) = walk.items[index].stop { send(.toggleGroup(id)) }
            case .stay: break
            }
            return .handled
        }
        if press.key == .return || press.key == .space, let at {
            switch walk.items[at].stop {
            case .row(let id): openTerminal(id, keepKeys: press.key == .space)
            case .group(let id): if Focus.rules.pressFoldsBand(space: press.key == .space) { send(.toggleGroup(id)) }
            }
            return .handled
        }
        guard press.modifiers.intersection([.command, .control]).isEmpty, let character = press.characters.first,
              !character.isWhitespace, !character.isNewline else { return .ignored }
        typed.add(character, at: Date())
        if let index = walk.typeAhead(from: at, typed.text) { cursor = walk.items[index].stop }
        return .handled
    }

    /// Show a terminal: ↩ and a click give it the keyboard (also when it is shown already); Space leaves it in the list.
    func openTerminal(_ id: TerminalID, keepKeys: Bool = false) {
        if keepKeys {
            if id != state.selected { hooks.browse() }
            send(.select(id))
        } else {
            send(.select(id))
            hooks.focusTerminal()
        }
    }
}
