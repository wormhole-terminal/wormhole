// The end of a window, and terminals moving between windows: a group's "Move to new window" takes its terminals along
// running; closing a window forgets it unless it is the last, which ends the app (saved).

import AppKit
import WindowsModel
import WorkspaceModel

extension AppController {
    /// A window of its own for terminals of `session`, which keep running in it, at `place` or a step from `session`'s. False when the workspace cannot hand
    /// terminals over (the sample) or none of them is here.
    @discardableResult
    func moveToNewWindow(_ ids: [TerminalID], from session: WindowSession, at place: WindowFrame? = nil) -> Bool {
        // The notes of a terminal that moves are not the notes of one that closed.
        session.notes?.departing(ids)
        guard let departure = session.hub.store.takeOut(ids) else { return false }
        let source = frame(of: session)
        let moved = makeSession(window: session.windows.backend.allocWindowId(), empty: true, in: session.windows)
        moved.hub.store.receive(departure)
        present(moved, frame: place ?? source.map(cascaded), activate: true)
        // The new window's record names the terminals before the old one is written without them.
        moved.saver?.saveNow()
        session.saver?.saveNow()
        return true
    }

    /// The person closes a window. The last one quits the app instead (saved, and asked like any quit); another asks first
    /// while something runs in it, and is forgotten once it closes.
    func shouldClose(_ session: WindowSession) -> Bool {
        guard shown.count > 1 else {
            NSApp.terminate(nil)
            return false
        }
        guard !asking(in: session).isEmpty else { return true }
        askToClose(session)
        return false
    }

    /// The red button's dot (the document-edited mark), on while closing the window would ask (`sync_close_dot`).
    func followCloseDot(of session: WindowSession) {
        let dot = withObservationTracking { !asking(in: session).isEmpty } onChange: { [weak self, weak session] in
            Task { @MainActor in if let session { self?.followCloseDot(of: session) } }
        }
        if session.window?.isDocumentEdited != dot { session.window?.isDocumentEdited = dot }
    }

    /// `kill` (SIGTERM) quits as ⌘Q does, saved and with the hosted terminals let go of, without the question
    /// (`app/sigterm.rs`).
    func quitOnSigterm() {
        signal(SIGTERM, SIG_IGN)
        let source = DispatchSource.makeSignalSource(signal: SIGTERM, queue: .main)
        source.setEventHandler { [weak self] in MainActor.assumeIsolated { self?.terminateUnattended() } }
        source.resume()
        sigterm = source
    }

    /// A window is gone: its terminals end and the core forgets it. Not while the app quits, and not the last one
    /// (closing it ends the app, saved): then it stays as it is.
    func windowClosed(_ session: WindowSession) {
        guard !terminating, shown.count > 1 else { return }
        session.saver?.cancel()
        session.hub.terminals.shutdown()
        session.windows.backend.forget(session.id)
        sessions.removeAll { $0 === session }
        if lastFront === session { lastFront = shown.last }
        menus?.windowsChanged()
    }

    /// Every window as it is, written now (a quit).
    func saveAllWindows() {
        sessions.forEach { $0.saver?.saveNow() }
        front.windows.backend.saveFront(front.id)
    }
}
