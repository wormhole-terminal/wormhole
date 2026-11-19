// A profile's settings as its windows and terminals show them: the keys of the theme, of the window's own look and of
// what a terminal does with the bell, Option and the clipboard. One place for both followers: the app's, for the profile
// Wormhole is in (`SettingsApplier`), and each other profile's with a window open (`ProfileLook`). Each key is applied
// only when its value moved (`changed`).

import DesignSystem
import FontPickerModel
import Foundation
import SettingsModel
import SidebarUI
import TerminalCore
import TerminalMetal
import WindowUI

@MainActor
enum LookSettings {
    /// The keys the theme store is made from (`ThemeSync.apply`).
    static let themeKeys = [
        "theme", "theme_light", "theme_mode", "theme_primary", "primary_color", "branch_color", "branch_color_custom", "borders",
        "settings_text_size", "ui_font_family", "ui_font_size", "settings_translucent", "studio_translucent", "hosts_translucent",
        "opacity", "blur",
    ]

    static func themeMoved(_ values: [String: JSONValue], since applied: [String: JSONValue]) -> Bool {
        themeKeys.contains { values[$0] != applied[$0] }
    }

    /// How the window itself looks. `keepsStatusBar`: the command line asked for the bar, which the setting then leaves
    /// alone; `automation`: the Automation feature is on in this profile (a row's menu offers "Hide from Agents");
    /// `motion`: the Motion feature is on in it.
    static func apply(
        _ values: [String: JSONValue], to look: WindowUI.WindowLook, changed: (String...) -> Bool, keepsStatusBar: Bool, automation: Bool, motion: Bool
    ) {
        if changed("opacity"), let value = values["opacity"]?.number { look.opacity = min(max(value, 0.3), 1) }
        if changed("blur") { look.blur = values["blur"]?.bool ?? look.blur }
        if changed("liquid_glass") { look.liquidGlass = values["liquid_glass"]?.bool ?? look.liquidGlass }
        if changed("status_bar"), !keepsStatusBar { look.statusBar = values["status_bar"]?.bool ?? false }
        if changed("smooth_resize") { look.smoothResize = values["smooth_resize"]?.bool ?? false }
        if changed("minimap") { look.minimap = values["minimap"]?.bool ?? false }
        if changed("icon_animations") { look.iconAnimations = values["icon_animations"]?.bool ?? true }
        if changed("split_gaps") { look.splitGaps = values["split_gaps"]?.bool ?? look.splitGaps }
        if changed("column_dividers") { look.columnDividers = values["column_dividers"]?.bool ?? look.columnDividers }
        // Without the Motion feature every column is Classic, whatever config.json holds (`motion_in_force`).
        if changed("motion.column", "feature_flags") {
            look.columnMotion = motion ? values["motion.column"]?.string ?? "classic" : "classic"
        }
        // Whether the Motion feature is on: what the window's open and close animation asks (`window_animation`).
        if look.motionFeature != motion { look.motionFeature = motion }
        if changed("terminals_in") { look.tabsOnTop = values["terminals_in"]?.string == "tabs" }
        if changed("density", "show_groups_as_cards", "fade_group_colors", "show_group_folders", "sidebar_notes", "sidebar_changes", "sidebar_changes_scope", "sidebar_status_dot", "sidebar_cmd_hints", "restore_terminal_text", "motion.status", "motion.insert", "motion.fold", "feature_flags") {
            look.sidebar = SidebarSettingsMaker.appearance(values, current: look.sidebar, motion: motion)
        }
        if look.sidebar.offersAgentHide != automation { look.sidebar.offersAgentHide = automation }
    }

    /// The code font (`code_font_family`, else the terminal's), for the window and for its theme's own windows.
    static func applyCodeFont(_ values: [String: JSONValue], folderRevision: Int, to look: WindowUI.WindowLook) {
        look.codeFont = PanelSettings.codeFont(values, folderRevision: folderRevision)
        look.themes.codeFont = look.codeFont
    }

    /// What a terminal does besides drawing: the bell's parts, which Option types Meta, what a program may do with the
    /// clipboard.
    static func apply(_ values: [String: JSONValue], to terminals: TerminalLook, changed: (String...) -> Bool) {
        if changed("bell_sound", "bell_visual") {
            terminals.use(
                bell: BellStyle(sound: values["bell_sound"]?.bool ?? true, visual: values["bell_visual"]?.bool ?? true, dock: false))
        }
        if changed("option_as_meta") {
            // Older config files have a boolean: `true` is both.
            let value = values["option_as_meta"]
            let side = value?.bool.map { $0 ? "both" : "off" } ?? value?.string
            terminals.use(optionAsMeta: side.flatMap(OptionAsMeta.init(setting:)) ?? .off)
        }
        if changed("clipboard_read", "clipboard_write") {
            terminals.use(
                clipboardRead: values["clipboard_read"]?.string.flatMap(ClipboardReadRule.init(rawValue:)) ?? .ask,
                write: values["clipboard_write"]?.string != "deny")
        }
    }
}
