// Carries what Settings holds into the live window, as it changes (a control, a hand edit of `config.json`, another
// window): the theme, how the window itself looks, the terminals' fonts, the sidebar's list, the title row and where the
// terminals are shown. Each key is applied only when its value moved, so a control of the control folder that set a look
// by hand is not undone by an unrelated setting.

import DesignSystem
import Foundation
import PanesModel
import PanesUI
import SettingsModel
import SettingsUI
import SidebarUI
import WindowUI

@MainActor
final class SettingsApplier {
    private unowned let app: AppController
    private let model: SettingsModel
    private var observation: SettingsObservation?
    private var applied: [String: JSONValue] = [:]

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
    func applyAmbient() { app.ambient?.apply(model.backend.values) { _ in true } }

    private func apply(_ values: [String: JSONValue]) {
        func changed(_ keys: String...) -> Bool { keys.contains { values[$0] != applied[$0] } }
        defer { applied = values }
        if changed("theme", "theme_light", "theme_mode"), app.options.theme == nil, app.options.appearance == nil {
            ThemeSync.apply(values)
        }
        if changed("font_family", "font_size", "line_height", "font_ligatures"), app.options.fontFamily == nil, app.options.pointSize == nil {
            app.terminalLook.use(FontSettingsMaker.settings(values, bundledDirectory: app.options.fontDirectory ?? Options.findBundledFonts()))
        }
        applyLook(values, changed: changed)
        app.ambient?.apply(values, changed: changed)
        if changed("agent_access", "control_write") { app.hooks?.settingsChanged() }
        if changed("title_row") { applyTitleRow(values) }
        if changed("open_in") {
            let chosen = values["open_in"]?.string ?? ""
            app.hub.openIn = SystemOpenIn(chosen: { chosen })
        }
    }

    private func applyLook(_ values: [String: JSONValue], changed: (String...) -> Bool) {
        let look = app.look
        if changed("opacity"), let value = values["opacity"]?.number { look.opacity = min(max(value, 0.3), 1) }
        if changed("blur") { look.blur = values["blur"]?.bool ?? look.blur }
        if changed("liquid_glass") { look.liquidGlass = values["liquid_glass"]?.bool ?? look.liquidGlass }
        if changed("status_bar"), !app.options.statusBar { look.statusBar = values["status_bar"]?.bool ?? false }
        if changed("minimap") { look.minimap = values["minimap"]?.bool ?? false }
        if changed("split_gaps") { look.splitGaps = values["split_gaps"]?.bool ?? look.splitGaps }
        if changed("column_dividers") { look.columnDividers = values["column_dividers"]?.bool ?? look.columnDividers }
        if changed("terminals_in") { look.tabsOnTop = values["terminals_in"]?.string == "tabs" }
        if changed("density", "show_groups_as_cards", "fade_group_colors", "show_group_folders", "sidebar_notes", "sidebar_changes", "restore_terminal_text", "motion.status") {
            look.sidebar = SidebarSettingsMaker.appearance(values, current: look.sidebar)
        }
    }

    private func applyTitleRow(_ values: [String: JSONValue]) {
        guard let keys = values["title_row"]?.array?.compactMap(\.string) else { return }
        let row = TitleRow(keys.compactMap(TitlePart.init(rawValue:)))
        guard row != app.hub.titleRow else { return }
        app.hub.titleRow = row
        app.hub.rowChanged()
    }
}
