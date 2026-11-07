// The palette's own keys, which the field hands over first (F-70): ↑↓ stop at the ends, Tab and ⇧Tab walk the scope
// row, ↩ runs, Esc clears, then goes back to everything, then closes, ⌫ in an empty narrowed field goes back to
// all and ⌘⌫ closes the selected terminal.

import PaletteModel
import SwiftUI
import WorkspaceModel

extension PaletteView {
    func handle(_ key: FieldKey, rows: [PaletteRow], selection: String?, context: OverlayContext) -> Bool {
        let selected = rows.compactMap(\.result).first { $0.id == selection }?.command
        switch key {
        case .escape:
            if !text.isEmpty { text = "" } else if scope != .all { setScope(.all) } else { context.close(nil) }
        case .backspaceWhenEmpty:
            guard scope != .all else { return false }
            setScope(.all)
        case .up: chosen = PaletteSelection.step(-1, from: chosen, in: rows)
        case .down: chosen = PaletteSelection.step(1, from: chosen, in: rows)
        case .tab(let forward): setScope(scope.step(forward: forward))
        case .enter:
            if let selected { run(selected, context: context) }
        case .commandDelete:
            guard let selected, selected.group == .terminals, case .select(let id)? = selected.action else {
                return false
            }
            send(.close(id))
            context.close(nil)
        }
        return true
    }

    func setScope(_ new: PaletteScope) {
        scope = new
        chosen = nil
    }

    /// A prefix typed into an unnarrowed palette narrows it and leaves the field.
    func typed(_ input: String) {
        chosen = nil
        guard scope == .all else { return }
        let (found, rest) = PaletteScope.parse(input)
        guard found != .all else { return }
        scope = found
        text = rest
    }

    /// Carries a result out. An action that opens an overlay waits for this one to be gone, or its own
    /// dismissal would close it again.
    func run(_ command: Command, context: OverlayContext) {
        RecentStore.shared.remember(command)
        if let id = command.appAction { return context.close { perform(id) } }
        guard let action = command.action else { return context.close(nil) }
        if case .showOverlay = action { return context.close { send(action) } }
        send(action)
        context.close(nil)
    }
}
