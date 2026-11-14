// Carries what Settings holds into the live window, as it changes (a control, a hand edit of `config.json`, another
// window): the theme, how the window itself looks, the terminals' fonts, the sidebar's list, the title row and where the
// terminals are shown. Each key is applied only when its value moved, so a control of the control folder that set a look
// by hand is not undone by an unrelated setting. These are the settings of the profile Wormhole is in, and they reach its
// windows only: a window of another profile follows its own (`ProfileLook`), with the same rules (`LookSettings`, `HubSettings`).

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
        if LookSettings.themeMoved(values, since: applied), app.options.theme == nil, app.options.appearance == nil {
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
        LookSettings.apply(values, to: app.terminalLook, changed: changed)
        LookSettings.apply(
            values, to: app.look, changed: changed, keepsStatusBar: app.options.statusBar,
            automation: model.host.flags().contains { $0.key == "automation" && $0.on },
            motion: model.host.flags().contains { $0.key == "motion" && $0.on })
        app.ambient?.apply(values, changed: changed)
        if changed("hotkey", "hotkey_action") { app.hotkey?.apply(spec: values["hotkey"]?.string ?? "", action: values["hotkey_action"]?.string ?? "") }
        if changed("jump_waiting_key") { app.bindJumpKey(values["jump_waiting_key"]?.string ?? "") }
        // The server also reads the Probes' flag, the folds' flag and "Never hide lines" again with them.
        if changed("agent_access", "control_write", "feature_flags", "never_hide_lines") {
            app.hooks?.settingsChanged(profile: app.windowsHost.profile)
            app.menus?.windowsChanged()
        }
        if changed("code_font_family", "code_font_size", "font_family") || fontsMoved {
            LookSettings.applyCodeFont(values, folderRevision: folder.revision, to: app.look)
        }
        if changed(
            "changes_tree", "group_untracked", "diff_split", "diff_wrap", "tab_width", "size_units", "worktree_base",
            "worktree_clone_build_folders")
        {
            PanelSettings.apply(values, to: ownHubs)
        }
        if changed("update_channel") { app.updateFollower?.use(channel: values["update_channel"]?.string == "unstable" ? .preview : .stable) }
        if changed("auto_update"), app.aux.hasUpdater {
            app.aux.updateBackend().apply(autoUpdate: values["auto_update"]?.bool ?? true)
        }
        if changed("sidebar_changes_remember") { app.windowsHost.backend.keepChanges(values["sidebar_changes_remember"]?.bool ?? true) }
        if changed("title_row") { HubSettings.applyTitleRow(values, zen: app.look.zen, to: ownHubs) }
        if changed("open_in") { HubSettings.applyOpenIn(values, to: ownHubs) }
        if changed("sidebar_numbers", "sidebar_changes_plain", "sidebar_changes_shared") { HubSettings.applySidebarRules(values, to: ownHubs) }
    }

    /// The windows in sight that are in the profile these settings are of.
    private var ownHubs: [WindowHub] { app.ownSessions.filter { !$0.parked }.map(\.hub) }

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
        HubSettings.dress(session.hub, model.backend.values, zen: app.look.zen)
        HubSettings.applyFeatures(off: app.featuresOff(session), to: session.hub)
        PanelSettings.link(session.hub, to: model)
    }

    /// Zen mode started or ended: each window's row is the preset's, or the person's own again.
    func applyZen() { HubSettings.applyTitleRow(model.backend.values, zen: app.look.zen, to: ownHubs) }
}
