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
        case .commandCopy:
            guard let selected, case .setting(let id)? = selected.target else { return false }
            sources.copySettingKey(id)
        case .commandEnter:
            guard let selected else { return false }
            openInNewWindow(selected, context: context)
        case .commandDelete:
            guard let selected, selected.group == .terminals, case .terminal(let id)? = selected.target else { return false }
            context.close { sources.closeTerminal(id) }
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
        defer { findSettings() }
        guard scope == .all else { return }
        let (found, rest) = PaletteScope.parse(input)
        guard found != .all else { return }
        scope = found
        text = rest
    }

    /// The settings the query finds, once per query or scope change.
    func findSettings() {
        let searching = scope == .all || scope == .settings
        hits = searching && !text.trimmingCharacters(in: .whitespaces).isEmpty ? sources.settings(text) : []
    }

    /// ⌘↩: a new window where the result is. A setting, an action, a project's action, a shell and a profile are not places:
    /// they run as ↩ does (core `run_in_new_window`).
    func openInNewWindow(_ command: Command, context: OverlayContext) {
        switch command.target {
        case .terminal?, .folder?, .commit?:
            RecentStore.shared.remember(command)
            context.close { sources.openInNewWindow(command.target!) }
        default: run(command, context: context)
        }
    }

    /// Carries a result out. An action that opens an overlay waits for this one to be gone, or its own
    /// dismissal would close it again.
    func run(_ command: Command, context: OverlayContext) {
        RecentStore.shared.remember(command)
        if let id = command.appAction { return context.close { sources.perform(id) } }
        if let target = command.target { return context.close { sources.reveal(target) } }
        guard let action = command.action else { return context.close(nil) }
        send(action)
        context.close(nil)
    }
}
