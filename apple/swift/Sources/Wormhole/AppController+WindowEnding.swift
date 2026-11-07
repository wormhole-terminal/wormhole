// The end of a window, and terminals moving between windows: a group's "Move to new window" takes its terminals along
// running; closing a window forgets it unless it is the last, which ends the app (saved).

import AppKit
import WindowsModel
import WorkspaceModel

extension AppController {
    /// A window of its own for terminals of `session`, which keep running in it. False when the workspace cannot hand
    /// terminals over (the sample) or none of them is here.
    @discardableResult
    func moveToNewWindow(_ ids: [TerminalID], from session: WindowSession) -> Bool {
        // The notes of a terminal that moves are not the notes of one that closed.
        session.notes?.departing(ids)
        guard let departure = session.hub.store.takeOut(ids) else { return false }
        let source = frame(of: session)
        let moved = makeSession(window: windowsHost.backend.allocWindowId(), empty: true)
        moved.hub.store.receive(departure)
        present(moved, frame: source.map(cascaded), activate: true)
        // The new window's record names the terminals before the old one is written without them.
        moved.saver?.saveNow()
        session.saver?.saveNow()
        return true
    }

    /// The person closes a window. The last one quits the app instead (saved, and asked like any quit); another asks first
    /// while something runs in it, and is forgotten once it closes.
    func shouldClose(_ session: WindowSession) -> Bool {
        guard sessions.count > 1 else {
            NSApp.terminate(nil)
            return false
        }
        let running = session.hub.state.sections.flatMap(\.rows).filter(\.runsSomething)
        guard !running.isEmpty else { return true }
        askToClose(session, running: running)
        return false
    }

    /// A window is gone: its terminals end and the core forgets it. Not while the app quits, and not the last one
    /// (closing it ends the app, saved): then it stays as it is.
    func windowClosed(_ session: WindowSession) {
        guard !terminating, sessions.count > 1 else { return }
        session.saver?.cancel()
        session.hub.terminals.shutdown()
        windowsHost.backend.forget(session.id)
        sessions.removeAll { $0 === session }
        if lastFront === session { lastFront = sessions.last }
        menus?.windowsChanged()
    }

    /// Every window as it is, written now (a quit).
    func saveAllWindows() {
        sessions.forEach { $0.saver?.saveNow() }
        windowsHost.backend.saveFront(front.id)
    }
}
