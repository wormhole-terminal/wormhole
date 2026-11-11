// A recorded shortcut checked by core's key rules (`shortcut_record`): the field's behaviour, what is taken by the
// app, macOS or the shell, and the spelling `config.json` stores.

import SettingsModel
import WormholeBindings

public struct RustShortcuts: ShortcutChecking {
    public init() {}

    public func check(key: String, spec: String) -> ShortcutCheck {
        switch checkShortcut(configKey: key, spec: spec) {
        case .accepted(let stored, _): .accepted(stored)
        case .cancel: .cancel
        case .clear: .clear
        case .needsModifier: .needsModifier
        case .refused(_, let reason), .invalid(let reason): .refused(reason)
        }
    }
}
