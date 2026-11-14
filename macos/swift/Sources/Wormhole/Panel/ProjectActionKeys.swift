// A project action's own key (`actions.json` `key`), taken by the window wherever the focus is, as the GPUI app's
// `claim_window_key` does (`app/handlers.rs`, `features/project/actions.rs` `action_for_key`): the terminal hears it through
// `claimProjectKey`, and `ProjectKeys` is the window-level handler for a key typed anywhere else (a text field, the
// sidebar, the panel). The key is matched against the actions the window already lists (cheap: a key that is nobody's asks
// nothing), then the core is asked about it each time (`ProjectStore.runAction(forKey:)`): a project that is no longer
// trusted, or whose Actions went off, lets the key through. The run itself goes through the core as well.

import AppKit
import DesignSystem
import PanelModel
import PanelUI
import PlatformModel
import WindowUI

extension AppController {
    /// Whether `key` (the core's name of it, with these modifiers) is the key of an action of the project the window's
    /// terminal is in; the action was started then. A project key carries ⌘ or is an F-key, so typing never asks.
    func claimProjectKey(_ key: String, _ flags: NSEvent.ModifierFlags, in hub: WindowHub) -> Bool {
        var mods: KeyMods = []
        if flags.contains(.shift) { mods.insert(.shift) }
        if flags.contains(.command) { mods.insert(.command) }
        if flags.contains(.control) { mods.insert(.control) }
        if flags.contains(.option) { mods.insert(.option) }
        return claimProjectKey(KeySpec(mods, key), in: hub)
    }

    /// `repeated`: the key is held and this is one of its repeats, which is the action's too (it never reaches the terminal, a
    /// field or a menu) but runs nothing again: one press, one run.
    func claimProjectKey(_ pressed: KeySpec, in hub: WindowHub, repeated: Bool = false) -> Bool {
        guard pressed.mods.contains(.command) || pressed.isFunctionKey, !hasActiveDialog(hub) else { return false }
        let project = hub.panel.project
        // The actions of the project of the terminal that has the keys (`action_for_key` asks of the window's active
        // terminal): the panel's list only while the panel reads that terminal's folder. It follows the selection a beat
        // later, and in that beat its list is the terminal before's, whose action must not run here.
        guard let folder = hub.state.selectedPath, project.folder == folder,
            project.state?.actions.contains(where: { $0.key.flatMap { KeySpec(glyphs: $0) }?.sameKey(pressed) == true }) == true
        else { return false }
        if repeated { return true }
        return project.runAction(forKey: pressed.glyphText)
    }

    /// Something in the window asks first and the key is its (GPUI's `has_active_dialog`): the palette or the shortcut sheet
    /// over the window, a sheet on it (a question, a worktree's), or a diff line's comment being written.
    private func hasActiveDialog(_ hub: WindowHub) -> Bool {
        hub.state.overlay != nil || hub.panel.changes.commenting != nil
            || sessions.first(where: { $0.hub === hub })?.window?.attachedSheet != nil
    }
}

/// The window-level handler: a keyDown in a window of the workspace that is an action's key runs it, whoever has the focus.
@MainActor
final class ProjectKeys {
    private var monitor: Any?
    private unowned let app: AppController

    init(app: AppController) { self.app = app }

    func start() {
        guard monitor == nil else { return }
        monitor = NSEvent.addLocalMonitorForEvents(matching: .keyDown) { [weak self] event in
            MainActor.assumeIsolated { self?.claims(event) ?? false } ? nil : event
        }
    }

    private func claims(_ event: NSEvent) -> Bool {
        // Only a window of the workspace: a sheet, Settings or the Studio read their own keys; and never a press for a shortcut
        // recorder (`RecordsKeys`), wherever one is.
        guard !event.goesToRecorder, let window = event.window, let session = app.sessions.first(where: { $0.window === window }),
            let pressed = Self.spec(of: event)
        else { return false }
        // A held key's repeats are taken here too, before the terminal or a menu sees them, and run nothing.
        return app.claimProjectKey(pressed, in: session.hub, repeated: event.isARepeat)
    }

    /// The press as the core names it: ⌘ held with a character, or an F-key.
    private static func spec(of event: NSEvent) -> KeySpec? {
        if let function = event.specialKey.flatMap({ functionKeys[$0] }) { return KeySpec(modifiers(of: event), function) }
        guard event.modifierFlags.contains(.command), let key = event.charactersIgnoringModifiers, key.count == 1 else { return nil }
        return KeySpec(modifiers(of: event), key)
    }

    private static func modifiers(of event: NSEvent) -> KeyMods {
        let flags = event.modifierFlags.intersection(.deviceIndependentFlagsMask)
        var mods: KeyMods = []
        if flags.contains(.shift) { mods.insert(.shift) }
        if flags.contains(.command) { mods.insert(.command) }
        if flags.contains(.control) { mods.insert(.control) }
        if flags.contains(.option) { mods.insert(.option) }
        return mods
    }

    private static let functionKeys: [NSEvent.SpecialKey: String] = [
        .f1: "f1", .f2: "f2", .f3: "f3", .f4: "f4", .f5: "f5", .f6: "f6", .f7: "f7", .f8: "f8", .f9: "f9", .f10: "f10",
        .f11: "f11", .f12: "f12", .f13: "f13", .f14: "f14", .f15: "f15", .f16: "f16", .f17: "f17", .f18: "f18", .f19: "f19",
        .f20: "f20",
    ]
}
