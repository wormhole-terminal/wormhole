// A project action's own key (`actions.json` `key`), taken by the window before the focused terminal hears it, as the GPUI
// app's `claim_window_key` does (`app/handlers.rs`, `features/project/actions.rs` `action_for_key`). The key is matched
// against the actions the core listed for the window's project, which it lists only while they may run, so no disk is
// touched for a key that is nobody's; the run itself goes through the core, which reads the project and its trust again
// (`ProjectService::run_action`) and refuses an action of a project that is not enabled and trusted at that moment.

import AppKit
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
        let pressed = KeySpec(mods, key)
        guard mods.contains(.command) || pressed.isFunctionKey else { return false }
        let project = hub.panel.project
        guard let action = project.state?.actions.first(where: { $0.key.flatMap { KeySpec(glyphs: $0) }?.sameKey(pressed) == true })
        else { return false }
        project.run(action)
        return true
    }
}
