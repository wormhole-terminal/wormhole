// Where the person's theme picks are written. A pick is a setting (`theme` for dark, `theme_light` for light), and what
// choosing, saving and renaming a theme does to them is the core's one rule (`config::picks`, shared with the GPUI
// app), so the Settings cards, the onboarding's Look step and the Theme Studio all set the same thing and none moves the
// mode. The settings backend implements this over the FFI; the store reads the result back as the settings change.

import Foundation

@MainActor
public protocol ThemePicking: AnyObject {
    /// A theme of the kind `dark` (or light) was chosen: it is the pick the mode says a choice sets.
    func pick(_ value: String, dark: Bool)
    /// A theme was saved in the file at `path`: nothing for a theme that is a pick already, else the pick of its
    /// own kind. Whether a pick changed.
    @discardableResult func pickAfterSave(_ value: String, dark: Bool, path: String) -> Bool
    /// A theme was renamed: a pick that is one of the names (old, new) becomes what replaces it.
    func renamePicks(_ renames: [(old: String, new: String)])
    /// Use with Match system: the light pick is `light`, the dark pick `dark`, and the mode follows macOS.
    func followSystem(light: String, dark: String)
    /// Whether the pick `setting` (a name, a slug or a path) names the theme saved in the file at `path`.
    func selects(_ setting: String, path: String) -> Bool
}
