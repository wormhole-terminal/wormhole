// Each terminal's overlay (find, size chip, bars), made with its view, and the verbs that reach it.

import TerminalUI
import WorkspaceModel

extension WindowHub {
    /// The overlay of terminal `id`, made on first use; nil while the terminal's view cannot start.
    func overlay(for id: TerminalID) -> TerminalOverlay? {
        if let known = overlays[id] { return known }
        guard let view = terminals.view(for: id) else { return nil }
        let bars = BarActions()
        bars.onClose = { [unowned self] in actions.closeTerminal(id) }
        bars.onRestart = { [unowned self] in restart(id) }
        let made = TerminalOverlay(view: view, actions: bars, notices: notices)
        overlays[id] = made
        return made
    }

    /// A shell that ended starts again in the same place: its view and overlay are made anew.
    public func restart(_ id: TerminalID) {
        overlays[id] = nil
        terminals.discard(id)
        restarts[id, default: 0] += 1
    }

    /// The selected terminal's overlay.
    var selectedOverlay: TerminalOverlay? { state.selected.flatMap(overlay(for:)) }

    /// A terminal's shell ended: its exited bar shows.
    public func noteExit(of id: TerminalID, code: Int32?) { overlay(for: id)?.exited(code: code) }

    /// The question on the selected terminal's bar answered, as its buttons do (the control folder's `clipboard-answer`).
    public func answerClipboard(_ choice: ClipboardChoice) { selectedOverlay?.actions.answerClipboard(choice) }

    public enum FindStep { case open, next, previous, useSelection }

    /// ⌘F, ⌘G, ⇧⌘G, ⌘E.
    public func find(_ step: FindStep) {
        guard let find = selectedOverlay?.find else { return }
        switch step {
        case .open: find.open()
        case .next: find.step(1)
        case .previous: find.step(-1)
        case .useSelection: find.useSelectionForFind()
        }
    }

    /// Overlays of terminals that are gone go with them.
    func forgetOverlays(keeping ids: Set<TerminalID>) {
        overlays = overlays.filter { ids.contains($0.key) }
    }
}

extension WindowHub {
    /// Types into the find bar's query, as the person would (the control folder's `find <text>`).
    public func setFindText(_ text: String) { selectedOverlay?.find.setText(text) }
}
