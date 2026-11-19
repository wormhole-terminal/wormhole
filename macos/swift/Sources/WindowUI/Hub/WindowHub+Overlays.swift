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
        bars.onReconnect = { [unowned self] in store.reconnect(id) }
        bars.onEndHost = { [unowned self] hard in endHost(id, hard: hard) }
        bars.onWait = { [unowned self] in
            overlays[id]?.waiting = true
            hostChanged(id)
        }
        bars.onDismissHostNotice = { [unowned self] in overlays[id]?.bars.hostNotice = nil }
        let made = TerminalOverlay(view: view, actions: bars, notices: notices)
        view.onFileLink = { [unowned self] text in openFile(text, in: id) }
        overlays[id] = made
        made.showHost(store.hostProblem(of: id))
        return made
    }

    /// The bar over terminal `id`'s pane follows what its session host is doing now.
    public func hostChanged(_ id: TerminalID) {
        overlays[id]?.showHost(store.hostProblem(of: id))
    }

    /// "End host": the terminal closes when the host ended, and the bar says why when it did not.
    public func endHost(_ id: TerminalID, hard: Bool) {
        overlays[id]?.endFailure = nil
        store.endHost(of: id, hard: hard) { [weak self] why in
            self?.overlays[id]?.endFailure = why
            self?.hostChanged(id)
        }
    }

    /// A shell that ended, or one that could not be shown, starts again in the same place: a new program (in a new host
    /// when they are on), its view and overlay made anew.
    public func restart(_ id: TerminalID) {
        store.startAgain(id)
        overlays[id] = nil
        terminals.discard(id)
        restarts[id, default: 0] += 1
    }

    /// The selected terminal's overlay.
    var selectedOverlay: TerminalOverlay? { state.selected.flatMap(overlay(for:)) }

    /// A terminal's shell ended: its exited bar shows.
    public func noteExit(of id: TerminalID, code: Int32?) {
        // The program ended with its host, which went away: the bar says so.
        let lost = store.hosting(of: id).map { if case .lost = $0 { true } else { false } } ?? false
        overlay(for: id)?.exited(code: code, hostLost: lost, shell: store.shellName(of: id))
        hostChanged(id)
    }

    /// The question on the selected terminal's bar answered, as its buttons do (the control folder's `clipboard-answer`).
    public func answerClipboard(_ choice: ClipboardChoice) { selectedOverlay?.actions.answerClipboard(choice) }

    public enum FindStep { case open, next, previous, useSelection }

    /// ⌘F, ⌘G, ⇧⌘G, ⌘E.
    public func find(_ step: FindStep) {
        guard let find = selectedOverlay?.find else { return }
        switch step {
        case .open: find.open(seed: find.selectionSeed)
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
