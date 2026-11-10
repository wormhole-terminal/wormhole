// A terminal let go outside its window (the Tear off feature, `window/tear_off.rs`): over the empty desktop it opens a window of
// its own there, over another Wormhole window it goes into that window's list, anywhere else nothing happens. AppKit
// sends a drag's events to the window it began in only, so what lies under the pointer is worked out from where the app's
// windows are, front to back; what that means is the core's (`tearOffLanding`, `tearOffWindow`). A terminal moves as a whole,
// running, and leaves its split behind. The last terminal of a window is never torn off into a window of its own, which
// would only move the window; it can still go into another window, and this one starts a new terminal.

import AppKit
import WindowUI
import WindowsModel
import WorkspaceModel

extension AppController {
    /// The terminal was let go with the pointer at `windowPoint` (the window's own space, y down). True when that is outside
    /// its window and the window took it; false when it is inside, where the panes and the list decide.
    func tearOffReleased(_ terminal: TerminalID, at windowPoint: CGPoint, from session: WindowSession) -> Bool {
        guard let window = session.window, let content = window.contentView else { return false }
        let local = content.isFlipped ? windowPoint : CGPoint(x: windowPoint.x, y: content.bounds.height - windowPoint.y)
        let global = window.convertPoint(toScreen: content.convert(local, to: nil))
        guard !window.frame.contains(global) else { return false }
        land(terminal, from: session, at: global)
        return true
    }

    /// What letting go of `terminal` at the global point `global` does. `covered` is whether another application's window is in
    /// front there; a script says it is not (nil: the system is asked).
    func land(_ terminal: TerminalID, from session: WindowSession, at global: CGPoint, covered: Bool? = nil) {
        // The feature's flag, and no outside in a window that is full screen (another Space lies there).
        guard settingsModel.host.flags().contains(where: { $0.key == "tear_off" && $0.on }),
            session.window?.styleMask.contains(.fullScreen) != true
        else { return }
        let ours = Set(NSApp.windows.map(\.windowNumber))
        let covered = covered ?? !ours.contains(NSWindow.windowNumber(at: global, belowWindowWithWindowNumber: 0))
        let stack = NSApp.orderedWindows.filter { $0.isVisible && !$0.isMiniaturized }.map { window in
            // Only a window in sight that shows this profile's terminals takes one; any other window of the app takes nothing.
            let taking = sessions.first { $0.window === window && !$0.parked && $0.windows.profile == session.windows.profile }
            return TearOffWindow(window: taking?.id, bounds: window.frame)
        }
        switch session.windows.backend.tearOffLanding(over: stack, at: global, from: session.id, covered: covered) {
        case .stay: break
        case .newWindow: tearOffToNewWindow(terminal, from: session, at: global)
        case .into(let id): if let target = sessions.first(where: { $0.id == id }) { move(terminal, from: session, into: target) }
        }
    }

    /// `terminal` opens a window of its own at the global point, kept inside the display it was let go on. Alone in its
    /// window it stays.
    private func tearOffToNewWindow(_ terminal: TerminalID, from session: WindowSession, at global: CGPoint) {
        guard session.hub.state.live.count > 1, let size = session.window?.frame.size,
            let screen = NSScreen.screens.first(where: { $0.frame.contains(global) })
        else { return }
        // The display's own space: from its top left, y down.
        let full = screen.frame, usable = screen.visibleFrame
        let visible = CGRect(x: usable.minX - full.minX, y: full.maxY - usable.maxY, width: usable.width, height: usable.height)
        let spot = CGPoint(x: global.x - full.minX, y: full.maxY - global.y)
        let placed = session.windows.backend.tearOffWindow(at: spot, size: size, visible: visible)
        let rect = NSRect(x: full.minX + placed.minX, y: full.maxY - placed.maxY, width: placed.width, height: placed.height)
        let number = screen.deviceDescription[NSDeviceDescriptionKey("NSScreenNumber")] as? NSNumber
        moveToNewWindow([terminal], from: session, at: WindowFrames.windowed(rect, on: number.map { UInt64($0.uint32Value) }))
    }

    /// `terminal` goes into another window's list, running; if that window is gone it stays where it was.
    private func move(_ terminal: TerminalID, from session: WindowSession, into target: WindowSession) {
        guard target !== session else { return }
        session.notes?.departing([terminal])
        guard let departure = session.hub.store.takeOut([terminal]) else { return }
        target.hub.store.receive(departure)
        target.saver?.saveNow()
        session.saver?.saveNow()
        if policy.touchesSystem { target.window?.makeKeyAndOrderFront(nil) }
    }
}
