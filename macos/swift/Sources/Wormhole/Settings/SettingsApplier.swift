// Carries what Settings holds into the live window, as it changes (a control, a hand edit of `config.json`, another
// window): the theme, how the window itself looks, the terminals' fonts, the sidebar's list, the title row and where the
// terminals are shown. Each key is applied only when its value moved, so a control of the control folder that set a look
// by hand is not undone by an unrelated setting.

import DesignSystem
import FileIconTheme
import FontPickerModel
import Foundation
import PanesModel
import PanesUI
import SettingsModel
#if WITH_UNIFFI
    import SettingsBridge
#endif
import SettingsUI
import SidebarUI
import TerminalCore
import TerminalMetal
import WorkspaceModel
import WindowUI

@MainActor
final class SettingsApplier {
    private unowned let app: AppController
    private let model: SettingsModel
    private var observation: SettingsObservation?
    private var applied: [String: JSONValue] = [:]
    private var appliedFolder = FontFolderState()

    init(app: AppController, model: SettingsModel) {
        self.app = app
        self.model = model
    }

    func start() {
        apply(model.backend.values)
        observation = model.backend.observe { [weak self] in
            guard let self else { return }
            self.apply(self.model.backend.values)
        }
    }

    /// The surfaces outside the window are made after the first apply: tell them what Settings holds.
    func applyAmbient() {
        app.ambient?.apply(model.backend.values) { _ in true }
        let values = model.backend.values
        app.hotkey?.apply(spec: values["hotkey"]?.string ?? "", action: values["hotkey_action"]?.string ?? "")
        app.bindJumpKey(values["jump_waiting_key"]?.string ?? "")
    }

    private func apply(_ values: [String: JSONValue]) {
        func changed(_ keys: String...) -> Bool { keys.contains { values[$0] != applied[$0] } }
        let folder = model.backend.fontFolder
        let fontsMoved = folder != appliedFolder
        defer { (applied, appliedFolder) = (values, folder) }
        if fontsMoved { ThemeStore.shared.fontFiles += 1 }
        if changed("theme", "theme_light", "theme_mode", "theme_primary", "primary_color", "branch_color", "branch_color_custom", "borders", "settings_text_size", "ui_font_family", "ui_font_size", "settings_translucent", "studio_translucent", "hosts_translucent", "opacity", "blur"), app.options.theme == nil, app.options.appearance == nil {
            ThemeSync.apply(values)
        }
        // The theme file in use, or one it extends, was edited by hand or by a script: the follower noticed, and
        // the keys did not move, so ask the store whether the files did.
        ThemeStore.shared.refreshIfThemeFilesChanged()
        if changed("icon_theme", "feature_flags") { applyIcons() }
        if changed("font_family", "font_size", "line_height", "font_ligatures") || fontsMoved, app.options.fontFamily == nil, app.options.pointSize == nil {
            app.terminalLook.use(
                FontSettingsMaker.settings(values, bundledDirectory: app.options.fontDirectory ?? Options.findBundledFonts(), folder: folder))
        }
        if changed("restore_terminal_text", "feature_flags") { app.sessions.forEach { $0.hub.store.source.settingsChanged() } }
        if changed("bell_sound", "bell_visual") {
            app.terminalLook.use(
                bell: BellStyle(
                    sound: values["bell_sound"]?.bool ?? true, visual: values["bell_visual"]?.bool ?? true, dock: false))
        }
        if changed("option_as_meta") {
            let value = values["option_as_meta"]
            let side = value?.bool.map { $0 ? "both" : "off" } ?? value?.string
            app.terminalLook.use(optionAsMeta: side.flatMap(OptionAsMeta.init(setting:)) ?? .off)
        }
        if changed("clipboard_read", "clipboard_write") {
            app.terminalLook.use(
                clipboardRead: values["clipboard_read"]?.string.flatMap(ClipboardReadRule.init(rawValue:)) ?? .ask,
                write: values["clipboard_write"]?.string != "deny")
        }
        applyLook(values, changed: changed)
        app.ambient?.apply(values, changed: changed)
        if changed("hotkey", "hotkey_action") { app.hotkey?.apply(spec: values["hotkey"]?.string ?? "", action: values["hotkey_action"]?.string ?? "") }
        if changed("jump_waiting_key") { app.bindJumpKey(values["jump_waiting_key"]?.string ?? "") }
        // The server also reads the Probes' flag, the folds' flag and "Never hide lines" again with them.
        if changed("agent_access", "control_write", "feature_flags", "never_hide_lines") {
            app.hooks?.settingsChanged(profile: app.windowsHost.profile)
            app.menus?.windowsChanged()
        }
        if changed("code_font_family", "code_font_size", "font_family") || fontsMoved {
            app.look.codeFont = PanelSettings.codeFont(values, folderRevision: folder.revision)
            ThemeStore.shared.codeFont = app.look.codeFont
        }
        if changed(
            "changes_tree", "group_untracked", "diff_split", "diff_wrap", "tab_width", "size_units", "worktree_base",
            "worktree_clone_build_folders")
        {
            PanelSettings.apply(values, to: app.shown.map(\.hub))
        }
        if changed("update_channel") { app.updateFollower?.use(channel: values["update_channel"]?.string == "unstable" ? .preview : .stable) }
        if changed("auto_update"), app.aux.hasUpdater {
            app.aux.updateBackend().apply(autoUpdate: values["auto_update"]?.bool ?? true)
        }
        if changed("sidebar_changes_remember") { app.windowsHost.backend.keepChanges(values["sidebar_changes_remember"]?.bool ?? true) }
        if changed("title_row") { applyTitleRow(values, to: app.shown.map(\.hub)) }
        if changed("open_in") { applyOpenIn(values, to: app.shown.map(\.hub)) }
        if changed("sidebar_numbers", "sidebar_changes_plain", "sidebar_changes_shared") { applySidebarRules(values, to: app.shown.map(\.hub)) }
    }

    /// The file icon theme in force: the setting, or the bundled one while the Icon themes flag is off (core's rule); "None"
    /// draws the system's symbols. Without the core the bundled theme stays.
    private func applyIcons() {
        #if WITH_UNIFFI
            guard let host = model.host as? RustSettingsHost else { return }
            let theme = CoreIconTheme(directory: host.handle.paths().iconThemesDir, name: host.iconThemeInForce())
            FileIconSource.shared.theme = theme.hasIcons ? theme : nil
        #endif
    }

    /// A window made after the settings were applied takes what they said.
    func dress(_ session: WindowSession) {
        let values = model.backend.values
        applyTitleRow(values, to: [session.hub])
        if values["open_in"] != nil { applyOpenIn(values, to: [session.hub]) }
        applySidebarRules(values, to: [session.hub])
        PanelSettings.apply(values, to: [session.hub])
        PanelSettings.link(session.hub, to: model)
    }

    /// What the core's workspace counts by: ⌘1–9 staying with their terminal or following the list as shown
    /// (`sidebar_numbers`), and which work a row's ±N is made of (`sidebar_changes_plain`, `sidebar_changes_shared`).
    private func applySidebarRules(_ values: [String: JSONValue], to hubs: [WindowHub]) {
        let follow = values["sidebar_numbers"]?.string == "list"
        let rules = WorkspaceAction.setChangeRules(
            plain: values["sidebar_changes_plain"]?.string ?? "finishing", shared: values["sidebar_changes_shared"]?.string ?? "each")
        for hub in hubs {
            hub.send(.setNumbersFollowList(follow))
            hub.send(rules)
        }
    }

    private func applyOpenIn(_ values: [String: JSONValue], to hubs: [WindowHub]) {
        let chosen = values["open_in"]?.string ?? ""
        hubs.forEach { $0.openIn = SystemOpenIn(chosen: { chosen }) }
    }

    private func applyLook(_ values: [String: JSONValue], changed: (String...) -> Bool) {
        let look = app.look
        if changed("opacity"), let value = values["opacity"]?.number { look.opacity = min(max(value, 0.3), 1) }
        if changed("blur") { look.blur = values["blur"]?.bool ?? look.blur }
        if changed("liquid_glass") { look.liquidGlass = values["liquid_glass"]?.bool ?? look.liquidGlass }
        if changed("status_bar"), !app.options.statusBar { look.statusBar = values["status_bar"]?.bool ?? false }
        if changed("smooth_resize") { look.smoothResize = values["smooth_resize"]?.bool ?? false }
        if changed("minimap") { look.minimap = values["minimap"]?.bool ?? false }
        if changed("icon_animations") { look.iconAnimations = values["icon_animations"]?.bool ?? true }
        if changed("split_gaps") { look.splitGaps = values["split_gaps"]?.bool ?? look.splitGaps }
        if changed("column_dividers") { look.columnDividers = values["column_dividers"]?.bool ?? look.columnDividers }
        if changed("terminals_in") { look.tabsOnTop = values["terminals_in"]?.string == "tabs" }
        if changed("density", "show_groups_as_cards", "fade_group_colors", "show_group_folders", "sidebar_notes", "sidebar_changes", "sidebar_changes_scope", "sidebar_status_dot", "sidebar_cmd_hints", "restore_terminal_text", "motion.status", "motion.insert", "motion.fold") {
            look.sidebar = SidebarSettingsMaker.appearance(values, current: look.sidebar)
        }
        let automation = model.host.flags().contains { $0.key == "automation" && $0.on }
        if look.sidebar.offersAgentHide != automation { look.sidebar.offersAgentHide = automation }
    }

    /// Zen mode started or ended: each window's row is the preset's, or the person's own again.
    func applyZen() { applyTitleRow(model.backend.values, to: app.shown.map(\.hub)) }

    private func applyTitleRow(_ values: [String: JSONValue], to hubs: [WindowHub]) {
        let zen = app.look.zen
        guard let keys = zen?.titleRow ?? values["title_row"]?.array?.compactMap(\.string) else { return }
        let row = TitleRow(keys.compactMap(TitlePart.init(rawValue:)))
        for hub in hubs {
            hub.zenActive = zen != nil
            guard hub.titleRow != row else { continue }
            hub.titleRow = row
            hub.rowChanged()
        }
    }
}
