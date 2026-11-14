// The look of a profile that is not the one Wormhole is in, while a window of it is open beside the others ("Open Window
// in", "Move Window to": `window/` in the GPUI app reads `settings_in(&self.profile)` everywhere). Such a window reads
// its OWN profile's settings, theme and fonts, never the shown profile's: this holds that profile's settings (followed
// live, as the app follows the shown one's), a theme store and a window look of its own, and the fonts and rules its
// terminals are dressed with. The profile Wormhole is in needs none: its windows wear the app's own look.

import DesignSystem
import FontPickerModel
import Foundation
import SettingsModel
import SettingsUI
import WindowUI
import WorkspaceModel

@MainActor
final class ProfileLook {
    let profile: String
    let look: WindowUI.WindowLook
    var themes: ThemeStore { look.themes }
    private(set) lazy var terminalLook = TerminalLook(
        terminals: { [weak self] in self?.sessions.map(\.hub.terminals) ?? [] }, themes: themes, engineProfile: { [profile = self.profile] in profile })
    private unowned let app: AppController
    let backend: any SettingsBackend
    /// The profile's own settings host: core's rules over its settings (the title row's parts, `TitleRowSetting`).
    let host: (any SettingsHost)?
    /// Whether a feature (by its flag's key) is on in this profile.
    private let featureOn: @MainActor (String) -> Bool
    private var observation: SettingsObservation?
    private var applied: [String: JSONValue] = [:]
    private var appliedFolder = FontFolderState()

    init(
        profile: String, app: AppController, backend: any SettingsBackend, host: (any SettingsHost)? = nil, themes: ThemeStore,
        featureOn: @escaping @MainActor (String) -> Bool
    ) {
        (self.profile, self.app, self.backend, self.host, self.featureOn) = (profile, app, backend, host, featureOn)
        look = WindowUI.WindowLook(themes: themes)
        look.zen = app.look.zen
    }

    /// The windows that wear this look.
    var sessions: [WindowSession] { app.sessions.filter { $0.dress.look === look } }

    func start() {
        apply(backend.values)
        terminalLook.followTheme()
        observation = backend.observe { [weak self] in
            guard let self else { return }
            self.apply(self.backend.values)
        }
    }

    /// A window made in the profile, or moved into it, after its settings were applied.
    func dress(_ session: WindowSession) {
        HubSettings.dress(session.hub, backend.values, zen: look.zen)
        // The panel's own switches are saved in the window's own profile.
        session.hub.panel.preferences.persist = { [backend] key, value in
            if backend.values[key]?.bool != value { backend.set(key, to: .bool(value)) }
        }
    }

    /// Zen mode started or ended: it is every window's, whichever profile it is in.
    func applyZen(_ zen: WindowShown?) {
        look.zen = zen
        HubSettings.applyTitleRow(backend.values, zen: zen, to: sessions.map(\.hub))
    }

    private func apply(_ values: [String: JSONValue]) {
        func changed(_ keys: String...) -> Bool { keys.contains { values[$0] != applied[$0] } }
        let folder = backend.fontFolder
        let fontsMoved = folder != appliedFolder
        defer { (applied, appliedFolder) = (values, folder) }
        // The type roles read the app's count (`TextRole.font`): registered font files are the whole process's.
        if fontsMoved { ThemeStore.shared.fontFiles += 1 }
        if LookSettings.themeMoved(values, since: applied) { ThemeSync.apply(values, to: themes) }
        themes.refreshIfThemeFilesChanged()
        if changed("font_family", "font_size", "line_height", "font_ligatures") || fontsMoved, app.options.fontFamily == nil, app.options.pointSize == nil {
            terminalLook.use(
                FontSettingsMaker.settings(values, bundledDirectory: app.options.fontDirectory ?? Options.findBundledFonts(), folder: folder))
        }
        LookSettings.apply(values, to: terminalLook, changed: changed)
        LookSettings.apply(
            values, to: look, changed: changed, keepsStatusBar: app.options.statusBar, automation: featureOn("automation"),
            motion: featureOn("motion"))
        if changed("code_font_family", "code_font_size", "font_family") || fontsMoved {
            LookSettings.applyCodeFont(values, folderRevision: folder.revision, to: look)
        }
        // Agent access settles in this profile too (a file edit of its `config.json` is its own event).
        if changed("agent_access", "control_write", "feature_flags", "never_hide_lines") { app.hooks?.settingsChanged(profile: profile) }
        let hubs = sessions.map(\.hub)
        if changed(
            "changes_tree", "group_untracked", "diff_split", "diff_wrap", "tab_width", "size_units", "worktree_base",
            "worktree_clone_build_folders")
        {
            PanelSettings.apply(values, to: hubs)
        }
        if changed("title_row") { HubSettings.applyTitleRow(values, zen: look.zen, to: hubs) }
        if changed("open_in") { HubSettings.applyOpenIn(values, to: hubs) }
        if changed("sidebar_numbers", "sidebar_changes_plain", "sidebar_changes_shared") { HubSettings.applySidebarRules(values, to: hubs) }
    }
}
