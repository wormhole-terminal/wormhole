// The list under the keys: ↑ ↓ ← → move the cursor and fold or open groups, ⌥↑ ⌥↓ go to the ends, letters jump by name,
// Return opens a terminal and Space folds a group (`SidebarWalk` decides each move). Esc ends a drag in flight.

import SwiftUI
import WorkspaceModel

extension SidebarView {
    func handle(_ press: KeyPress, _ walk: SidebarWalk) -> KeyPress.Result {
        if press.key == .escape, drag != nil {
            drag = nil
            outside = false
            terminalDrag?.leave()
            return .handled
        }
        if press.key == .escape, searching || searchOpen {
            endSearch()
            return .handled
        }
        keysLead = true
        let at = walk.index(of: cursor)
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
            case .row(let id): send(.select(id))
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
}
