// Putting a window on the screen: the windows saved at the last run each where it was, another one (⌘N, `wh new-window`),
// and the NSWindow's events on their way to the session.

import AppKit
import TerminalMetal
import WindowUI
import WindowsModel
import WorkspaceModel

extension AppController {
    /// The windows saved at the last run, each where it was (brought back onto a display that is there); with none, one
    /// window as a first launch has it. The core's order brings the window that was in front last forward again.
    func openSavedWindows() throws {
        let saved = windowsHost.backend.saved()
        let windows = saved.isEmpty ? [SavedWindow(id: windowsHost.backend.allocWindowId(), frame: nil, terminals: 0)] : saved
        for window in windows {
            let session = makeSession(window: window.id)
            let frame = window.frame.map { windowsHost.backend.fit($0, on: WindowFrames.displays()) }
            // One that was in the Dock opens behind, unfocused, and goes back there: it is never the one that comes forward,
            // which is the last of the others (`window/window_bounds.rs` `front_after_restore`).
            guard present(session, frame: frame, activate: frame?.minimised != true) else {
                throw Options.UsageError(description: "no terminal source could start")
            }
        }
    }

    /// A window with nothing but a new terminal, laid out like the one in front, a step from it (⌘N, the Dock's menu).
    func openNewWindow(folder: String? = nil) {
        let planned = windowsHost.backend.plan(from: front.hub.store.source, folder: folder)
        openPlanned(window: planned.window, activate: true)
    }

    /// The window the core planned (`plan`, or `wh new-window`): its workspace is filled from the plan. It opens a step
    /// from the one in front; behind the others and without the keys unless `activate`.
    func openPlanned(window id: WindowID, activate: Bool, in windows: WindowsHost? = nil) {
        let source = frame(of: front)
        let session = makeSession(window: id, in: windows)
        present(session, frame: source.map(cascaded), activate: activate)
    }

    /// Put the session's window on the screen. False when no terminal could be shown (a source did not start).
    @discardableResult
    func present(_ session: WindowSession, frame: WindowFrame?, activate: Bool) -> Bool {
        let hub = session.hub
        let first = hub.state.selected.flatMap { hub.terminals.view(for: $0) }
        if first == nil, !hub.state.live.isEmpty { return false }
        // A fixed grid is a measuring mode: the window fits it. The title row and the status bar sit above and below the
        // terminal: 38 + 24 pt. Otherwise the GPUI app's default window (1320 x 820): both columns open leave the
        // terminal the room its title row needs.
        let size = options.grid.flatMap { grid in
            first.map { view in
                let terminal = view.renderer.pointSize(forGrid: grid.cols, grid.rows)
                let pad = 2 * MetalRenderer.padding
                return CGSize(width: terminal.width + pad, height: terminal.height + pad + 62)
            }
        } ?? CGSize(width: 1320, height: 820)
        let title = "Wormhole (Swift experiment)" + (sessions.first === session ? "" : " \(session.id)")
        let controller = WorkspaceWindowController(
            title: title, contentSize: size, minSize: windowsHost.backend.minSize, content: WorkspaceRootView(hub: hub, look: look))
        wire(controller, of: session)
        session.controller = controller
        if let frame, let window = controller.window { WindowFrames.place(frame, on: window) }
        // A torn-off terminal's window grows from its chip: it starts as the chip, before it is shown.
        if let start = growing, let window = controller.window {
            growing = nil
            session.grow = TearOffGrow(window: window, start: start, appearance: look.sidebar, backend: session.windows.backend) { [weak session] in
                session?.grow = nil
            }
        }
        // The workspace's first word on the window: it is not in front yet (a test instance never is).
        controller.reportState()
        if activate { lastFront = session }
        if options.background || !activate {
            // A test instance, or a window a script asked for: behind the other windows, never the active app.
            controller.window?.orderBack(nil)
        } else {
            controller.showWindow(nil)
            NSApp.activate()
        }
        session.grow?.run()
        if let frame, let window = controller.window {
            if frame.minimised { window.miniaturize(nil) }
            WindowFrames.settle(frame, on: window)
        }
        session.saver?.start()
        session.saver?.saveNow()
        menus?.windowsChanged()
        return true
    }

    func wire(_ controller: WorkspaceWindowController, of session: WindowSession) {
        controller.onClose = { [weak self, weak session] in
            if let session { self?.windowClosed(session) }
        }
        controller.shouldClose = { [weak self, weak session] in
            guard let session else { return true }
            return self?.shouldClose(session) ?? true
        }
        controller.gridFacts = { [weak session] in session?.hub.gridFacts }
        controller.snapsToGrid = { [weak self] in !(self?.look.smoothResize ?? false) }
        controller.onState = { [weak self, weak session] active, seen in
            guard let session else { return }
            session.hub.store.windowState(active: active, seen: seen)
            if active { self?.cameToFront(session) }
        }
        controller.onFrameChange = { [weak session] in session?.saver?.frameChanged() }
    }

    /// Where `session`'s window is, in the core's space.
    func frame(of session: WindowSession) -> WindowFrame? { session.saver?.frame() }

    /// A new window's place: a step from the one it was opened from, on that window's display.
    func cascaded(_ frame: WindowFrame) -> WindowFrame {
        let displays = WindowFrames.displays()
        let home = displays.first { $0.id == frame.display } ?? displays.first
        return home.map { windowsHost.backend.cascade(frame, on: $0) } ?? frame
    }

    func cameToFront(_ session: WindowSession) {
        lastFront = session
        session.windows.backend.saveFront(session.id)
        menus?.windowsChanged()
    }
}
