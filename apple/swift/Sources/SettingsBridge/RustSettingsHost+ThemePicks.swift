// What choosing, saving and renaming a theme does to the picks and the mode is core's rule (`settings/theme_picks.rs`,
// `config::picks`): these only ask, and tell the model its values moved (core wrote them itself, so no wake says so).

import Foundation
import WormholeBindings

extension RustSettingsHost {
    public func pickTheme(_ value: String, dark: Bool) {
        if handle.pickTheme(value: value, dark: dark) { backend.reload() }
    }

    @discardableResult
    public func pickThemeAfterSave(_ value: String, dark: Bool, path: String) -> Bool {
        let changed = handle.pickThemeAfterSave(value: value, dark: dark, path: path)
        if changed { backend.reload() }
        return changed
    }

    public func renameThemePicks(_ renames: [(old: String, new: String)]) {
        if handle.renameThemePicks(renames: renames.flatMap { [$0.old, $0.new] }) { backend.reload() }
    }

    public func followSystemWith(light: String, dark: String) {
        if handle.followSystemWith(light: light, dark: dark) { backend.reload() }
    }

    public func themeSelects(_ setting: String, path: String) -> Bool { handle.themeSelects(setting: setting, path: path) }

    /// The icon theme in force: the setting, or the bundled one while the Icon themes flag is off.
    public func iconThemeInForce() -> String { handle.iconThemeInForce() }
}
