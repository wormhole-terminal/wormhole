// One window of the app: its workspace's hub, the factory of its terminals, what its panel needs and its `NSWindow`.
// The app keeps one per window (`AppController.sessions`); what is the app's alone (menus, settings, the hook socket)
// is not here.

import AppKit
import PanelUI
import WindowUI
import WindowsModel
import WorkspaceModel

@MainActor
final class WindowSession {
    let id: WindowID
    let hub: WindowHub
    let factory: TerminalFactory
    /// What the project's actions ask of this window, and the core's usage service (real windows only).
    let panelEffects: AnyObject?
    let panelUsage: AnyObject?
    /// What the right panel reads through (the core's where real).
    let panelBackends: PanelUI.PanelBackends
    var notes: NotesCoordinator?
    var controller: WorkspaceWindowController?
    var saver: WindowSaver?

    init(id: WindowID, hub: WindowHub, factory: TerminalFactory, effects: AnyObject?, usage: AnyObject?, backends: PanelUI.PanelBackends) {
        (self.id, self.hub, self.factory, panelEffects, panelUsage, panelBackends) = (id, hub, factory, effects, usage, backends)
    }

    var window: NSWindow? { controller?.window }

    /// Whether terminal `id` is one of this window's.
    func holds(_ terminal: TerminalID) -> Bool { hub.state.live.contains(terminal) }
}
