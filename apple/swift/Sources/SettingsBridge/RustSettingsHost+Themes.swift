// The theme files of the profile, from the Appearance cards: Import theme and Move to Trash. Core decides what is a theme, which
// picks a trashed one changes and what the question says (`settings/theme_files.rs`); the Trash is the system's.

import DesignSystem
import Foundation
import SettingsModel
import WormholeBindings

extension RustSettingsHost {
    public func importTheme(path: String) -> String? {
        do {
            try handle.importTheme(path: path)
            backend.reload()  // core's own write: no wake says the values moved
            return nil
        } catch {
            return SettingsFailure.sentence(error)
        }
    }

    public func themeTrashPlan(_ value: String) -> ThemeTrashQuestion? {
        handle.themeTrashPlan(value: value).map { ThemeTrashQuestion(message: $0.message, detail: $0.detail) }
    }

    public func trashTheme(_ value: String) -> Bool {
        guard let plan = handle.themeTrashPlan(value: value) else { return false }
        // The picks that name the file go back to the defaults first, so the app never falls back with an error.
        let before = handle.resetThemePicks(value: value)
        backend.reload()
        do {
            try Trash.move(URL(fileURLWithPath: plan.path))
            backend.reload()
            return true
        } catch {
            handle.restoreThemePicks(before: before)
            backend.reload()
            return false
        }
    }
}
