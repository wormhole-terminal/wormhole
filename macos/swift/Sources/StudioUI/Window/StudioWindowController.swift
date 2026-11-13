// The Studio's window: full-size content under a transparent title bar (the title row is ours, 38 high, with the
// traffic lights in it), 1344 × 900 and no smaller than 1300 × 640, closing through the session so unsaved edits are
// asked about first. One at a time: opening it again brings it forward (`toggle_studio`).

import AppKit
import DesignSystem
import StudioModel
import SwiftUI

@MainActor
public final class StudioWindowController: NSObject, NSWindowDelegate {
    private var window: NSWindow?
    private var session: StudioSession?
    private lazy var undoManager = StudioUndoManager { [unowned self] in self.session!.document }
    /// The panel and pasteboard of Export JSON and Copy JSON; a check replaces them with a scratch folder's.
    public var outputs = StudioOutputs.system { didSet { session?.outputs = outputs } }

    /// Opens the Studio on `theme` (what is in use), or brings the open one forward. `extras` draws the gallery's specimens
    /// of the modules above this one, by id (the window's status bar, icon tabs and resize handle).
    public func show<Extras: View>(
        theme: String, environment: StudioEnvironment, store: ThemeStore = .shared,
        @ViewBuilder extras: @escaping @MainActor (String) -> Extras
    ) throws {
        if let window {
            TestInstance.bring(window)
            return
        }
        let session = try StudioSession(theme: theme, environment: environment, store: store)
        let window = NSWindow(
            contentRect: CGRect(origin: .zero, size: StudioMetrics.windowSize),
            styleMask: [.titled, .closable, .miniaturizable, .resizable, .fullSizeContentView], backing: .buffered, defer: false)
        window.titleVisibility = .hidden
        window.titlebarAppearsTransparent = true
        window.minSize = StudioMetrics.windowMin
        window.title = StudioWords.title
        window.setFrameAutosaveName("StudioWindow") // text: an autosave name
        window.contentView = NSHostingView(rootView: StudioView(session: session, store: store, extras: extras).secondaryBackdrop(.studio, store: store))
        window.delegate = self
        window.isReleasedWhenClosed = false
        session.window = window
        session.outputs = outputs
        (self.window, self.session) = (window, session)
        window.center()
        TestInstance.bring(window)
    }

    public var isOpen: Bool { window != nil }

    /// Closes the window as the red light does: unsaved edits are asked about first (⌥⌘, with the Studio open).
    public func close() { window?.performClose(nil) }

    /// The open window's session, for a driver that has no pointer (the control folder's `studio-do`).
    public var activeSession: StudioSession? { session }

    /// The draft in the open window, for a driver that has no pointer (the control folder's `studio-*`).
    public var document: StudioDocument? { session?.document }

    /// Save theme (⌘S) and Duplicate, for the menu bar and the palette: no-ops while the Studio is closed.
    public func saveTheme() async { await session?.save() }
    public func duplicateTheme() { session?.duplicate() }
    public func exportJSON() async { await session?.exportJSON() }
    public func copyJSON() { session?.copyJSON() }

    /// The Edit menu's Undo and Redo are the draft's steps while this window is in front.
    public func windowWillReturnUndoManager(_ window: NSWindow) -> UndoManager? { session == nil ? nil : undoManager }

    public func windowShouldClose(_ sender: NSWindow) -> Bool {
        guard let session else { return true }
        Task {
            if await session.requestClose() { sender.delegate = nil; sender.close(); (window, self.session) = (nil, nil) }
        }
        return false
    }
}
